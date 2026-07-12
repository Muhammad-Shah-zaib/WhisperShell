export class CustomSelect extends HTMLElement {
  constructor() {
    super();
    this.options = [];
    this.value = '';
    this.isOpen = false;
    this._handleOutside = this._handleOutside.bind(this);
  }

  get value() {
    return this.getAttribute('value') || '';
  }

  set value(v) {
    this.setAttribute('value', v);
  }

  static get observedAttributes() {
    return ['options', 'value'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (oldValue !== newValue) {
      if (name === 'options') {
        try {
          this.options = JSON.parse(newValue);
        } catch (e) {
          console.error("Invalid options JSON", e);
        }
      }
      // We don't need to manually set this.value = newValue because we use getter/setter to attribute
      this.render();
    }
  }

  connectedCallback() {
    if (!this.value && this.options.length > 0) {
      this.value = this.options[0].value;
    }
    this.render();
    document.addEventListener('click', this._handleOutside);
  }

  disconnectedCallback() {
    document.removeEventListener('click', this._handleOutside);
  }

  _handleOutside(e) {
    if (!this.contains(e.target) && this.isOpen) {
      this.isOpen = false;
      const dropdown = this.querySelector('.custom-select-dropdown');
      if (dropdown) dropdown.classList.remove('open');
    }
  }

  get selectedOption() {
    // strict string comparison since attribute values are strings
    return this.options.find(o => String(o.value) === String(this.value)) || this.options[0];
  }

  render() {
    const sel = this.selectedOption;
    this.innerHTML = `
      <style>
        .custom-select-wrapper {
          position: relative;
          width: 100%;
          cursor: pointer;
          user-select: none;
        }
        .custom-select-trigger {
          display: flex;
          align-items: center;
          justify-content: space-between;
          width: 100%;
          color: var(--silver);
          font-size: 14px;
          padding-bottom: 8px;
          border-bottom: 1px solid var(--border);
          transition: color 0.2s ease, border-color 0.2s ease;
        }
        /* When wrapper is hovered, or when open */
        .custom-select-wrapper:hover .custom-select-trigger,
        .custom-select-wrapper.open .custom-select-trigger {
          color: var(--lavender);
          border-color: var(--lavender);
        }
        .custom-select-wrapper.open .custom-select-trigger .iconify {
          transform: rotate(180deg);
        }
        .custom-select-dropdown {
          position: absolute;
          top: 100%;
          left: -12px;
          right: -12px;
          background: var(--bg-card);
          border: 1px solid var(--border);
          border-radius: var(--radius);
          margin-top: 12px;
          z-index: 999;
          display: none;
          flex-direction: column;
          box-shadow: 0 8px 24px rgba(0,0,0,0.8);
          max-height: 250px;
          overflow-y: auto;
        }
        .custom-select-dropdown.open {
          display: flex;
        }
        .custom-select-option {
          padding: 12px 16px;
          color: var(--silver);
          font-size: 14px;
          transition: background 0.2s ease, color 0.2s ease;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .custom-select-option:hover:not(.disabled) {
          background: var(--hover-bg);
          color: var(--lavender);
        }
        .custom-select-option.disabled {
          color: rgba(138, 130, 158, 0.4); /* Much more muted */
          cursor: not-allowed;
        }
        .custom-select-option.selected {
          color: var(--lavender);
          background: var(--hover-bg);
        }
        .status-tag {
          font-size: 11px;
          background: rgba(180, 160, 220, 0.1);
          color: var(--text-muted);
          padding: 2px 6px;
          border-radius: 4px;
        }
        .download-btn, .cancel-btn {
          font-size: 11px;
          padding: 4px 8px;
          border-radius: 4px;
          border: none;
          cursor: pointer;
          transition: background 0.2s;
        }
        .download-btn {
          background: rgba(180, 160, 220, 0.15);
          color: var(--lavender);
        }
        .download-btn:hover {
          background: rgba(180, 160, 220, 0.3);
        }
        .cancel-btn {
          background: rgba(255, 100, 100, 0.15);
          color: #ff8888;
        }
        .cancel-btn:hover {
          background: rgba(255, 100, 100, 0.3);
        }
        .progress-text {
          font-size: 11px;
          color: var(--silver);
          min-width: 28px;
          text-align: right;
        }
        .progress-container {
          display: flex;
          align-items: center;
          gap: 8px;
          width: 180px;
        }
        .progress-bar-bg {
          flex: 1;
          height: 4px;
          background: rgba(0, 0, 0, 0.5); /* Dark background */
          border-radius: 2px;
          overflow: hidden;
        }
        .progress-bar-fill {
          height: 100%;
          background: var(--lavender);
          width: 0%;
          transition: width 0.1s linear;
        }
      </style>
      <div class="custom-select-wrapper ${this.isOpen ? 'open' : ''}">
        <div class="custom-select-trigger">
          <span class="trigger-label">${sel ? sel.label : 'Select an option'}</span>
          <span class="iconify" data-icon="mdi:chevron-down" style="font-size: 20px; transition: transform 0.2s ease;"></span>
        </div>
        <div class="custom-select-dropdown ${this.isOpen ? 'open' : ''}">
          ${this.options.map(o => `
            <div class="custom-select-option ${o.value === this.value ? 'selected' : ''} ${(o.disabled || o.downloading) ? 'disabled' : ''}" data-value="${o.value}">
              <span>${o.label}</span>
              ${o.downloading ? `
                <div class="progress-container">
                  <span class="progress-text">${Math.round(o.progress || 0)}%</span>
                  <div class="progress-bar-bg">
                    <div class="progress-bar-fill" style="width: ${o.progress || 0}%"></div>
                  </div>
                  <button class="cancel-btn" data-action="cancel" data-model="${o.value}">Cancel</button>
                </div>
              ` : (o.disabled ? `<button class="download-btn" data-action="download" data-model="${o.value}">Download</button>` : '')}
            </div>
          `).join('')}
        </div>
      </div>
    `;

    const wrapper = this.querySelector('.custom-select-wrapper');
    const dropdown = this.querySelector('.custom-select-dropdown');
    
    wrapper.addEventListener('click', (e) => {
      e.stopPropagation();
      this.isOpen = !this.isOpen;
      wrapper.classList.toggle('open', this.isOpen);
      dropdown.classList.toggle('open', this.isOpen);
    });

    const optionEls = this.querySelectorAll('.custom-select-option');
    optionEls.forEach(el => {
      el.addEventListener('click', (e) => {
        e.stopPropagation();
        
        const actionBtn = e.target.closest('[data-action]');
        if (actionBtn) {
          const action = actionBtn.dataset.action;
          const modelId = actionBtn.dataset.model;
          if (action === 'download') {
            this.dispatchEvent(new CustomEvent('download', { detail: { value: modelId }, bubbles: true, composed: true }));
          } else if (action === 'cancel') {
            this.dispatchEvent(new CustomEvent('cancelDownload', { detail: { value: modelId }, bubbles: true, composed: true }));
          }
          return;
        }

        if (el.classList.contains('disabled')) return;
        
        this.isOpen = false;
        this.value = el.dataset.value;
        this.dispatchEvent(new CustomEvent('change', { 
          detail: { value: this.value },
          bubbles: true,
          composed: true
        }));
      });
    });
  }

  updateProgress(modelId, progress) {
    // Update internal state silently
    const opt = this.options.find(o => String(o.value) === String(modelId));
    if (opt) {
      opt.progress = progress;
      opt.downloading = true;
    }
    
    // Selectively update DOM elements instead of re-rendering everything
    const optionEl = this.querySelector(`.custom-select-option[data-value="${modelId}"]`);
    if (optionEl) {
      const fill = optionEl.querySelector('.progress-bar-fill');
      const text = optionEl.querySelector('.progress-text');
      if (fill) fill.style.width = `${progress}%`;
      if (text) text.textContent = `${Math.round(progress)}%`;
    }
  }
}
