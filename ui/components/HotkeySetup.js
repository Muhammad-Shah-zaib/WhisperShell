export class HotkeySetup extends HTMLElement {
  constructor() {
    super();
    this.innerHTML = `
      <style>
        .accordion-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          cursor: pointer;
          font-size: 16px;
          font-weight: 600;
          color: var(--text-primary);
          transition: color 0.2s ease;
        }
        
        .accordion-header:hover {
          color: var(--lavender);
        }

        .accordion-header:hover .icon {
          color: var(--lavender);
        }

        .accordion-content {
          max-height: 0;
          overflow: hidden;
          transition: max-height 0.3s ease-out, margin-top 0.3s ease-out;
        }
        
        .accordion-content.open {
          max-height: 200px;
          margin-top: 16px;
        }
        
        .accordion-content ul {
          padding-left: 20px;
          color: var(--silver);
          font-size: 14px;
          line-height: 1.6;
        }

        .icon {
          color: var(--text-primary);
          font-size: 20px;
          transition: transform 0.3s ease, color 0.2s ease;
        }

        .icon.open {
          transform: rotate(90deg);
        }
      </style>
      <div class="section-container">
        <div class="accordion-header" id="hotkey-header">
          <span>How to set up your Hotkey</span>
          <span class="iconify icon" id="hotkey-icon" data-icon="mdi:chevron-right"></span>
        </div>
        <div class="accordion-content" id="hotkey-content">
          <ul>
            <li>Open your system preferences (e.g., GNOME Settings).</li>
            <li>Navigate to Keyboard shortcuts.</li>
            <li>Add a new custom shortcut mapped to this exact command:<br>
              <code style="background: var(--bg-primary); padding: 2px 6px; border-radius: 4px; color: var(--lavender); margin-top: 4px; display: inline-block;">whispershell --toggle-recording</code>
            </li>
            <li>Use a memorable combination like <strong>Ctrl + Space</strong>.</li>
          </ul>
        </div>
      </div>
    `;

    const header = this.querySelector('#hotkey-header');
    const content = this.querySelector('#hotkey-content');
    const icon = this.querySelector('#hotkey-icon');

    header.addEventListener('click', () => {
      content.classList.toggle('open');
      icon.classList.toggle('open');
    });
  }
}
