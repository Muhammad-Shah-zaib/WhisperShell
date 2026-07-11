# IMPORTANT: WhisperShell Agent Reference Guide

**READ THIS FILE BEFORE STARTING ANY NEW WORK OR DEBUGGING SESSIONS.**

This project has highly specific architectural constraints. Failure to adhere to these rules will result in catastrophic, hard-to-trace bugs and extreme frustration.

## 1. Tauri v2 vs v1 APIs
This project operates on **Tauri v2**. The API paths have changed from v1.
- **CRITICAL:** Do NOT destructure `invoke` or `event` directly from `window.__TAURI__`.
- Always use this safe path fallback in vanilla JS to prevent silent UI crashes:
  ```javascript
  const invoke = window.__TAURI__.core ? window.__TAURI__.core.invoke : window.__TAURI__.invoke;
  const event = window.__TAURI__.event || window.__TAURI__.core?.event;
  ```

## 2. Frontend Logs Do NOT Show in the Rust Terminal
When running `npm run tauri dev`, the terminal you are watching only prints Rust `println!` statements. **JavaScript `console.log()` outputs will NOT appear in the terminal.** They are trapped in the WebView's DevTools.
- **CRITICAL:** To print frontend debug information to the terminal, you MUST use the provided `rustLog` wrapper.
- Do not use `console.log()` if you expect the user to see it in their terminal output.
  ```javascript
  // WRONG:
  console.log("Saving form data"); 

  // CORRECT:
  rustLog("Saving form data"); 
  ```

## 3. Web Components and Attribute Binding
The frontend is built using Vanilla JS and native Web Components (`HTMLElement`). It does not use React, Vue, or Alpine.js.
- **CRITICAL:** Web Components do not magically bind DOM attributes (`value="..."`) to class properties (`this.value`). 
- If you need a component to sync its state when `setAttribute` is called externally, you MUST explicitly define a getter and setter for the property:
  ```javascript
  get value() { return this.getAttribute('value') || ''; }
  set value(v) { this.setAttribute('value', v); }
  ```
- If you modify state that affects rendering (like `isOpen = false`), do it *before* you update values that trigger a DOM re-render, otherwise the component will re-render in the wrong state.

## 4. Native Disabled Buttons and Event Swallowing
When applying validation states to buttons (e.g., disabling a "Save" button when there are no form changes):
- **CRITICAL:** Do NOT use the native HTML `disabled="true"` attribute. Doing so intercepts the cursor and blocks all `click` events at the browser level.
- We require click events to fire even when a button is "disabled" so that we can log a "No changes detected. Action aborted." message via `rustLog`.
- Use the CSS `.disabled` class to handle styling, and manually check for it in the click listener:
  ```javascript
  document.querySelector('.btn-primary').addEventListener('click', async (e) => {
    if (e.currentTarget.classList.contains('disabled')) {
      rustLog('No changes detected in the form. Save aborted.');
      return; // Manually abort
    }
    // Proceed with save...
  });
  ```

## 5. Wayland and X11
The user is running Fedora with Wayland, which employs strict security isolation.
- Be aware that certain cross-process or global behaviors (like global hotkeys) are severely limited by Wayland.
- The backend explicitly forces `GDK_BACKEND=x11` (XWayland) to enable global hotkey support. Do not attempt to bypass this unless specifically requested by the user.

## 6. Tauri v2 Capabilities and Permissions (CRITICAL)
Tauri v2 employs a strict, capability-based security model. Core window APIs that were previously available by default in v1 must now be explicitly whitelisted.
- **The Trap:** If you attempt to call a window method (e.g., `appWindow.startDragging()`) and it fails silently, **it is likely missing a permission.**
- **The Fix:** You MUST manually add the capability to `src-tauri/capabilities/*.json` (like `default.json`).
- Example: To allow dragging the window via JavaScript, `"core:window:allow-start-dragging"` must be present in the `permissions` array.

## 7. Linux Window Manager Restrictions
- **Snapping/Tiling:** On Linux desktop environments like GNOME, the window manager (Mutter) strictly enforces that **non-resizable windows cannot be tiled or snapped** (e.g., using `Super + Arrow` keys). If you set `"resizable": false` in `tauri.conf.json`, you will break the user's OS-level window snapping shortcuts. Always set `"resizable": true` if the user relies on tiling shortcuts.
