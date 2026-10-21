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

  /* ---------- 2. 精确联合区域检测 ---------- */
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

    // 宠物与输入框之间的透明间隙
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

  /* ---------- 6. 渲染逻辑 ---------- */
  function applyMood(mood) {
    pet.classList.remove("mood-idle", "mood-happy", "mood-talking");
    pet.classList.add(`mood-${mood}`);
    // const img = document.getElementById("pet-img");
    // img.src = `/pet/${mood}.png`;
  }

  let hideTimer = null;
  function showBubble(text, isError) {
    bubble.textContent = text;
    bubble.classList.toggle("error", !!isError);
    bubble.classList.remove("hidden");
    if (hideTimer) clearTimeout(hideTimer);
    const ms = Math.min(8000, Math.max(2500, text.length * 120));
    hideTimer = setTimeout(() => bubble.classList.add("hidden"), ms);
  }

  await listen("pet://talking", (e) => applyMood(e.payload ? "talking" : "idle"));
  await listen("pet://state", (e) => applyMood(e.payload.mood));
  await listen("pet://speech", (e) => showBubble(String(e.payload), false));
  await listen("pet://chat_error", (e) => showBubble(String(e.payload), true));

  const initial = await invoke("get_state");
  applyMood(initial.mood);
}

bootstrap().catch((err) => console.error("pet bootstrap failed:", err));
