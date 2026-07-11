export class OverlaySize extends HTMLElement {
  constructor() {
    super();
    this.value = 'small'; // default
    this.innerHTML = `
      <style>
        .size-buttons {
          display: flex;
          gap: 16px;
          margin-bottom: 12px;
        }
        .size-btn {
          width: 48px;
          height: 48px;
          border-radius: 50%;
          border: 2px solid var(--border);
          background: transparent;
          color: var(--text-muted);
          font-size: 14px;
          font-weight: 600;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          transition: all 0.2s ease;
          user-select: none;
        }
        .size-btn:hover {
          border-color: var(--lavender);
          color: var(--text-primary);
          background: rgba(180, 160, 220, 0.05);
        }
        .size-btn:active {
          transform: scale(0.9);
        }
        .size-btn.active {
          border-color: var(--lavender);
          background: var(--lavender);
          color: var(--black);
        }
      </style>
      <div class="section-container">
        <div class="section-title">Overlay Size</div>
        <div class="section-desc">Size of the recording overlay on your screen.</div>
        
        <div class="size-buttons">
          <button class="size-btn active" data-size="small">SM</button>
          <button class="size-btn" data-size="medium">MD</button>
          <button class="size-btn" data-size="large">LG</button>
          <button class="size-btn" data-size="xlarge">XL</button>
        </div>
        
        <div style="margin-top: 16px;">
          <div class="field-label" style="margin-bottom: 12px;">Live Preview</div>
          <div style="display: flex; align-items: center; justify-content: center; height: 120px; padding: 20px; background: rgba(0,0,0,0.2); border-radius: 8px; overflow: hidden; position: relative;">
            
            <div id="preview-pill" style="
                background: var(--bg-card);
                border: 1px solid var(--border);
                border-radius: 999px;
                padding: 6px 12px;
                display: flex;
                align-items: center;
                gap: 12px;
                width: fit-content;
                box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
                height: 36px;
            ">
              <div style="width: 18px; height: 18px; display: flex; align-items: center; justify-content: center;">
                <img src="icons/whispershell-icon.svg" style="width: 100%; height: 100%; object-fit: contain;" alt="Logo" />
              </div>
              <div style="display: flex; align-items: center; gap: 3px; height: 16px;">
                <div style="width: 3px; height: 14px; border-radius: 2px; background: var(--accent); opacity: 0.9;"></div>
                <div style="width: 3px; height: 10px; border-radius: 2px; background: var(--accent); opacity: 0.9;"></div>
                <div style="width: 3px; height: 16px; border-radius: 2px; background: var(--accent); opacity: 0.9;"></div>
              </div>
            </div>

          </div>
        </div>
      </div>
    `;

    const previewPill = this.querySelector('#preview-pill');
    const buttons = this.querySelectorAll('.size-btn');

    this.applyScale = (val) => {
      if (val === 'small') {
        previewPill.style.zoom = '1';
      } else if (val === 'medium') {
        previewPill.style.zoom = '1.25';
      } else if (val === 'large') {
        previewPill.style.zoom = '1.5';
      } else if (val === 'xlarge') {
        previewPill.style.zoom = '1.75';
      }
    };

    const updateButtons = (val) => {
      buttons.forEach(btn => {
        if (btn.dataset.size === val) {
          btn.classList.add('active');
        } else {
          btn.classList.remove('active');
        }
      });
    };

    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const size = btn.dataset.size;
        this.value = size;
        this.setAttribute('value', size);
        updateButtons(size);
        this.applyScale(size);
        
        // Dispatch change event so index.html can checkChanges()
        this.dispatchEvent(new CustomEvent('change', {
          bubbles: true,
          composed: true
        }));
      });
    });
  }

  // To support initial setup via setAttribute from index.html
  static get observedAttributes() {
    return ['value'];
  }

  attributeChangedCallback(name, oldValue, newValue) {
    if (name === 'value' && oldValue !== newValue) {
      this.value = newValue;
      const buttons = this.querySelectorAll('.size-btn');
      buttons.forEach(btn => {
        if (btn.dataset.size === newValue) {
          btn.classList.add('active');
        } else {
          btn.classList.remove('active');
        }
      });
      if (this.applyScale) {
          this.applyScale(newValue);
      }
    }
  }
}
