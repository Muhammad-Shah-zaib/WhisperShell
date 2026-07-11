export class FilePaths extends HTMLElement {
  constructor() {
    super();
    this.innerHTML = `
      <style>
        .browse-btn {
          background: transparent;
          border: none;
          color: var(--silver);
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 4px;
          border-radius: 4px;
          transition: background 0.2s ease, color 0.2s ease;
        }
        .browse-btn:hover {
          background: var(--hover-bg);
          color: var(--lavender);
        }
        .field-row-flex {
          display: flex;
          align-items: center;
          border-bottom: 1px solid var(--border);
          padding-bottom: 8px;
          transition: border-color 0.2s ease;
        }
        .field-row-flex:hover {
          border-color: var(--lavender);
        }
        .field-row-flex:hover input,
        .field-row-flex:hover .icon-main {
          color: var(--lavender);
        }
        .field-row-flex .icon-main {
          color: var(--silver);
          font-size: 20px;
          margin-right: 12px;
          transition: color 0.2s ease;
        }
        .field-row-flex input {
          flex: 1;
        }
      </style>
      <div class="section-container">
        <div class="section-title" style="margin-bottom: 16px;">File Paths</div>
        
        <div>
          <div class="field-label">Voice Recordings Directory</div>
          <div class="field-row-flex">
            <span class="iconify icon-main" data-icon="mdi:folder-outline"></span>
            <input type="text" id="voice-recordings-dir" value="~/.local/share/whispershell/recordings" />
            <button class="browse-btn copy-btn" title="Copy path">
              <span class="iconify" data-icon="mdi:content-copy" style="font-size: 18px;"></span>
            </button>
            <button class="browse-btn browse-dir-btn" title="Browse directory" style="margin-left: 4px;">
              <span class="iconify" data-icon="mdi:folder-search-outline" style="font-size: 18px;"></span>
            </button>
          </div>
        </div>

        <div style="margin-top: 16px;">
          <div class="field-label">Last Messages Log</div>
          <div class="field-row-flex">
            <span class="iconify icon-main" data-icon="mdi:file-document-outline"></span>
            <input type="text" id="messages-log-file" value="~/.local/share/whispershell/messages.log" />
            <button class="browse-btn copy-btn" title="Copy path">
              <span class="iconify" data-icon="mdi:content-copy" style="font-size: 18px;"></span>
            </button>
            <button class="browse-btn browse-dir-btn" title="Browse file" style="margin-left: 4px;">
              <span class="iconify" data-icon="mdi:file-search-outline" style="font-size: 18px;"></span>
            </button>
          </div>
        </div>

        <div style="margin-top: 16px;">
          <div class="field-label">Error Logs Directory</div>
          <div class="field-row-flex">
            <span class="iconify icon-main" data-icon="mdi:folder-alert-outline"></span>
            <input type="text" id="error-logs-dir" value="~/.local/state/whispershell/errors" />
            <button class="browse-btn copy-btn" title="Copy path">
              <span class="iconify" data-icon="mdi:content-copy" style="font-size: 18px;"></span>
            </button>
            <button class="browse-btn browse-dir-btn" title="Browse directory" style="margin-left: 4px;">
              <span class="iconify" data-icon="mdi:folder-search-outline" style="font-size: 18px;"></span>
            </button>
          </div>
        </div>
      </div>
    `;

    const invoke = window.__TAURI__.core ? window.__TAURI__.core.invoke : window.__TAURI__.invoke;

    const voiceDirBtn = this.querySelectorAll('.browse-dir-btn')[0];
    const messagesBtn = this.querySelectorAll('.browse-dir-btn')[1];
    const errorDirBtn = this.querySelectorAll('.browse-dir-btn')[2];

    const voiceDirInput = this.querySelector('#voice-recordings-dir');
    const messagesInput = this.querySelector('#messages-log-file');
    const errorDirInput = this.querySelector('#error-logs-dir');

    voiceDirBtn.addEventListener('click', async () => {
      const path = await invoke('select_directory');
      if (path) voiceDirInput.value = path;
    });

    messagesBtn.addEventListener('click', async () => {
      const path = await invoke('select_file');
      if (path) messagesInput.value = path;
    });

    errorDirBtn.addEventListener('click', async () => {
      const path = await invoke('select_directory');
      if (path) errorDirInput.value = path;
    });

    // Add click listeners to copy buttons
    const copyBtns = this.querySelectorAll('.copy-btn');
    const inputs = [voiceDirInput, messagesInput, errorDirInput];
    
    copyBtns.forEach((btn, index) => {
      btn.addEventListener('click', async () => {
        const path = inputs[index].value;
        if (path) {
          try {
            await navigator.clipboard.writeText(path);
            
            // Temporary visual feedback
            const icon = btn.querySelector('.iconify');
            const originalIcon = icon.getAttribute('data-icon');
            icon.setAttribute('data-icon', 'mdi:check');
            icon.style.color = 'var(--lavender)';
            
            setTimeout(() => {
              icon.setAttribute('data-icon', originalIcon);
              icon.style.color = '';
            }, 1500);
            
            await invoke('log_to_terminal', { msg: "Copied to clipboard: " + path });
          } catch (err) {
            await invoke('log_to_terminal', { msg: "Failed to copy: " + err });
          }
        }
      });
    });
  }
}
