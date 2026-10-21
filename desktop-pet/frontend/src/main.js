import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

async function bootstrap() {
  const appWindow = getCurrentWindow();
  const pet = document.getElementById("pet");
  const bubble = document.getElementById("bubble");
  const input = document.getElementById("chat-input");

  let isClickThrough = false;
  let isHovering = false;

  /* ---------- 1. 输入框显隐控制 ---------- */
  function updateInputVisibility() {
    if (isHovering && !isClickThrough) {
      input.classList.remove("hidden");
    } else {
      input.classList.add("hidden");
      input.blur(); // 如果鼠标移开，主动失去焦点
    }
  }

  /* ---------- 2. 精确联合区域检测（宠物 + 输入框 + 缝隙） ---------- */
  window.addEventListener("mousemove", (e) => {
    if (isClickThrough) return;

    const petRect = pet.getBoundingClientRect();
    const inputRect = input.getBoundingClientRect();

    const inPet =
      e.clientX >= petRect.left && e.clientX <= petRect.right &&
      e.clientY >= petRect.top && e.clientY <= petRect.bottom;

    const inInput =
      e.clientX >= inputRect.left && e.clientX <= inputRect.right &&
      e.clientY >= inputRect.top && e.clientY <= inputRect.bottom;

    // 使用 inputRect.bottom 到 petRect.top 作为判断间隙
    const inGap =
      e.clientX >= Math.min(petRect.left, inputRect.left) &&
      e.clientX <= Math.max(petRect.right, inputRect.right) &&
      e.clientY > inputRect.bottom && e.clientY < petRect.top;

    const newHovering = inPet || inInput || inGap;

    if (newHovering !== isHovering) {
      isHovering = newHovering;
      updateInputVisibility();
    }
  });

  /* ---------- 3. 穿透模式切换 (Ctrl+Shift+P) ---------- */
  window.addEventListener("keydown", async (e) => {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      isClickThrough = !isClickThrough;

      await appWindow.setIgnoreCursorEvents(isClickThrough);

      if (isClickThrough) {
        document.body.classList.add("click-through");
        isHovering = false;
      } else {
        document.body.classList.remove("click-through");
      }
      updateInputVisibility();
      console.log("Click-through mode:", isClickThrough);
    }
  });

  /* ---------- 4. 鼠标事件交互 ---------- */
  pet.addEventListener("mousedown", async (e) => {
    if (e.button !== 0 || isClickThrough) return;
    await appWindow.startDragging();
  });

  pet.addEventListener("click", async (e) => {
    if (e.detail === 0 || isClickThrough) return;
    await invoke("pet_clicked");
  });

  /* ---------- 5. 焦点获取与中文输入 ---------- */
  // 点击输入框时，强制让 Tauri 窗口从系统抢过焦点，唤醒输入法
  input.addEventListener("mousedown", async (e) => {
    e.stopPropagation(); // 防止触发拖拽等事件
    await appWindow.setFocus();
    input.focus();
  });

  input.addEventListener("keydown", async (e) => {
    // 解决中文输入法拦截问题：如果正在输入法中选词，直接返回
    if (e.key === "Enter" && e.isComposing) {
      return;
    }
    // 确认发送
    if (e.key === "Enter") {
      const text = input.value.trim();
      if (!text) return;
      input.value = "";
      try {
        await invoke("chat", { text });
      } catch (err) {
        console.error("chat invoke failed:", err);
      }
    }
  });

  /* ---------- 6. 状态与气泡渲染（含流式打字机效果） ---------- */
  function applyMood(mood) {
    pet.classList.remove("mood-idle", "mood-happy", "mood-talking");
    pet.classList.add(`mood-${mood}`);

    // 如果有三态图片，可以放开以下注释
    // const img = document.getElementById("pet-img");
    // img.src = `/pet/${mood}.png`;
  }

  let hideTimer = null;
  let currentChatText = "";

  function showBubble(text, isError, autoHide = true) {
    bubble.textContent = text;
    bubble.classList.toggle("error", !!isError);
    bubble.classList.remove("hidden");

    if (hideTimer) clearTimeout(hideTimer);
    if (autoHide) {
      const ms = Math.min(8000, Math.max(2500, text.length * 120));
      hideTimer = setTimeout(() => bubble.classList.add("hidden"), ms);
    }
  }

  // 基础状态监听
  await listen("pet://talking", (e) => applyMood(e.payload ? "talking" : "idle"));
  await listen("pet://state", (e) => applyMood(e.payload.mood));

  // 聊天开始：清空气泡，准备接收流式数据
  await listen("pet://chat_start", () => {
    currentChatText = "";
    showBubble("", false, false);
  });

  // 接收流式 token：实时更新气泡文字，不自动消失
  await listen("pet://chat_token", (e) => {
    currentChatText += String(e.payload);
    showBubble(currentChatText, false, false);
  });

  // 完整回复到达（流式结束），设定计时器自动隐藏气泡
  await listen("pet://speech", (e) => {
    const finalText = String(e.payload);
    if (!currentChatText) {
      // 如果没有触发流式，直接显示完整文本
      showBubble(finalText, false, true);
    } else {
      // 流式结束，复用当前气泡，设定自动隐藏
      const ms = Math.min(8000, Math.max(2500, finalText.length * 120));
      if (hideTimer) clearTimeout(hideTimer);
      hideTimer = setTimeout(() => bubble.classList.add("hidden"), ms);
    }
  });

  // 错误显示
  await listen("pet://chat_error", (e) => {
    const msg = String(e.payload);
    // 如果是 mock 模式提示，显示 6 秒
    if (msg.includes("mock 模式")) {
      showBubble(msg, true, false);
      hideTimer = setTimeout(() => bubble.classList.add("hidden"), 6000);
    } else {
      showBubble(msg, true, true);
    }
  });

  /* ---------- 7. 首次拉取状态 ---------- */
  const initial = await invoke("get_state");
  applyMood(initial.mood);
}

bootstrap().catch((err) => console.error("pet bootstrap failed:", err));