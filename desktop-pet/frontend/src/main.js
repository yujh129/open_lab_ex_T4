import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * 入口引导：所有 await 收敛在 async 函数内，避开 vite target=es2021
 * 不支持顶层 await 的限制。
 */
async function bootstrap() {
  const pet = document.getElementById("pet");
  const bubble = document.getElementById("bubble");
  const input = document.getElementById("chat-input");

  /* ---------- 1. 拖拽 ---------- */
  pet.addEventListener("mousedown", async (e) => {
    if (e.button !== 0) return;
    await getCurrentWindow().startDragging();
  });

  /* ---------- 2. 单击宠物 ---------- */
  pet.addEventListener("click", async (e) => {
    if (e.detail === 0) return;
    await invoke("pet_clicked");
  });

  /* ---------- 3. 输入框回车 → 调 chat command ---------- */
  input.addEventListener("keydown", async (e) => {
    if (e.key !== "Enter") return;
    if (e.isComposing) return; // 输入法候选确认时不提交
    const text = input.value.trim();
    if (!text) return;
    input.value = "";
    try {
      await invoke("chat", { text });
    } catch (err) {
      // Rust 已 emit pet://chat_error，这里只兜底记录
      console.error("chat invoke failed:", err);
    }
  });

  /* ---------- 4. 状态与气泡渲染 ---------- */
  function applyMood(mood) {
    pet.classList.remove("mood-idle", "mood-happy", "mood-talking");
    pet.classList.add(`mood-${mood}`);
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

  await listen("pet://talking", (event) => {
    applyMood(event.payload ? "talking" : "idle");
  });

  await listen("pet://state", (event) => {
    applyMood(event.payload.mood);
  });

  await listen("pet://speech", (event) => {
    showBubble(String(event.payload), false);
  });

  await listen("pet://chat_error", (event) => {
    showBubble(String(event.payload), true);
  });

  /* ---------- 5. 首次拉取状态 ---------- */
  const initial = await invoke("get_state");
  applyMood(initial.mood);
}

bootstrap().catch((err) => {
  console.error("pet bootstrap failed:", err);
});
