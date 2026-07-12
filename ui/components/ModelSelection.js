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
    const event = window.__TAURI__.event || window.__TAURI__.core?.event;
    
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

    let options = knownModels.map(m => {
      const isInstalled = installedFiles.includes(m.filename);
      return {
        value: m.id,
        label: m.label,
        filename: m.filename,
        disabled: !isInstalled,
        downloading: false,
        progress: 0
      };
    });

    const select = this.querySelector('#model-select');
    select.setAttribute('options', JSON.stringify(options));

    // Cleanup previous listeners if we reload
    if (this._unlistenProgress) this._unlistenProgress();
    if (this._unlistenComplete) this._unlistenComplete();

    this._unlistenProgress = await event.listen('download_progress', (e) => {
      const { model_id, progress } = e.payload;
      // Directly update the specific progress bar to prevent full DOM re-rendering (glitching)
      if (select && select.updateProgress) {
        select.updateProgress(model_id, progress);
      }
      
      // Keep local state in sync
      options = options.map(o => {
        if (o.value === model_id) {
          return { ...o, downloading: true, progress };
        }
        return o;
      });
    });

    this._unlistenComplete = await event.listen('download_complete', (e) => {
      const model_id = e.payload;
      options = options.map(o => {
        if (o.value === model_id) {
          return { ...o, downloading: false, disabled: false, progress: 0 };
        }
        return o;
      });
      select.setAttribute('options', JSON.stringify(options));
      // Optionally auto-select the newly downloaded model if it's the only one or if the user wanted it
      select.value = model_id;
      select.dispatchEvent(new CustomEvent('change', { detail: { value: model_id }, bubbles: true, composed: true }));
    });

    select.addEventListener('download', async (e) => {
      const modelId = e.detail.value;
      const model = options.find(o => o.value === modelId);
      if (model) {
        options = options.map(o => o.value === modelId ? { ...o, downloading: true, progress: 0 } : o);
        select.setAttribute('options', JSON.stringify(options));
        try {
          await invoke('download_model', { modelId, filename: model.filename });
        } catch (err) {
          console.error("Download failed or cancelled:", err);
          // Revert state
          options = options.map(o => o.value === modelId ? { ...o, downloading: false, progress: 0 } : o);
          select.setAttribute('options', JSON.stringify(options));
        }
      }
    });

    select.addEventListener('cancelDownload', async (e) => {
      try {
        await invoke('cancel_download');
      } catch (err) {
        console.error("Failed to cancel download:", err);
      }
    });
  }
}
