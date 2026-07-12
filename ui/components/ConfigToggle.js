export class ConfigToggle extends HTMLElement {
  constructor() {
    super();
    this.innerHTML = `
      <style>
        .accordion-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          cursor: pointer;
          font-family: 'Poppins', sans-serif;
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
          display: inline-block;
          transition: transform 0.3s ease, color 0.2s ease;
        }

        .icon.open {
          transform: rotate(90deg);
        }
      </style>
      <div class="section-container">
        <div class="accordion-header" id="config-header">
          <span>How to bring this window back</span>
          <span class="iconify icon" id="config-icon" data-icon="mdi:chevron-right"></span>
        </div>
        <div class="accordion-content" id="config-content">
          <ul>
            <li>You can safely close this window (X); WhisperShell will continue running in the background.</li>
            <li style="margin-top: 8px;">To bring this configuration window back up, you can either:</li>
            <li style="margin-top: 4px; margin-left: 12px;"><strong>1.</strong> Launch the WhisperShell app again from your desktop or application menu.</li>
            <li style="margin-top: 4px; margin-left: 12px;"><strong>2.</strong> Run the following command in your terminal:</li>
            <li style="margin-top: 4px; margin-left: 24px;">
              <code style="display: inline-block;">whispershell --toggle-config</code>
            </li>
            <li style="margin-top: 8px;"><strong>Pro Tip:</strong> Just like the recording hotkey, you can bind this command to a custom keyboard shortcut in your system settings to pop your config up instantly from anywhere!</li>
          </ul>
        </div>
      </div>
    `;

    const header = this.querySelector('#config-header');
    const content = this.querySelector('#config-content');
    const icon = this.querySelector('#config-icon');

    header.addEventListener('click', () => {
      content.classList.toggle('open');
      icon.classList.toggle('open');
    });
  }
}
