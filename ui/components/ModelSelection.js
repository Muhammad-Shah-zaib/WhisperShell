export class ModelSelection extends HTMLElement {
  constructor() {
    super();
    this.innerHTML = `
      <div class="section-container">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
          <div class="section-title" style="margin-bottom: 0;">Model Selection</div>
          <span id="active-model-badge" class="badge" style="margin-left: 0;">
            <span class="model-name">Loading Model...</span>
            <span class="model-status-dot yellow"></span>
          </span>
        </div>
        <div class="section-desc">Choose the dictation model that best balances speed and accuracy for your hardware.</div>
        
        <custom-select id="model-select"></custom-select>
      </div>
    `;
    
    this.loadModels();
  }

  async loadModels() {
    const invoke = window.__TAURI__.core ? window.__TAURI__.core.invoke : window.__TAURI__.invoke;
    
    // Default known models
    const knownModels = [
      { id: "base", filename: "ggml-base.en.bin", label: "Whisper Base" },
      { id: "turbo", filename: "ggml-large-v3-turbo.bin", label: "Whisper Turbo" },
      { id: "large", filename: "ggml-large-v3.bin", label: "Whisper Large" },
      { id: "parakeet", filename: "parakeet-v3.bin", label: "Parakeet v3" }
    ];

    let installedFiles = [];
    try {
      installedFiles = await invoke('get_installed_models');
    } catch (e) {
      console.error("Failed to fetch installed models:", e);
    }

    const options = knownModels.map(m => {
      const isInstalled = installedFiles.includes(m.filename);
      return {
        value: m.id,
        label: m.label,
        disabled: !isInstalled,
        // Optional: Custom select can handle extra fields if we updated it, but for now we'll just append text if we want,
        // actually CustomSelect already checks `o.disabled` and appends a tag. We'll update CustomSelect to say "Need to be downloaded".
      };
    });

    const select = this.querySelector('#model-select');
    select.setAttribute('options', JSON.stringify(options));
  }
}
