import { CustomSelect } from './components/CustomSelect.js';
import { ModelSelection } from './components/ModelSelection.js';
import { HotkeySetup } from './components/HotkeySetup.js';
import { ConfigToggle } from './components/ConfigToggle.js';
import { FilePaths } from './components/FilePaths.js';
import { HistoryLimit } from './components/HistoryLimit.js';
import { OverlaySize } from './components/OverlaySize.js';

customElements.define('custom-select', CustomSelect);
customElements.define('model-selection', ModelSelection);
customElements.define('hotkey-setup', HotkeySetup);
customElements.define('config-toggle', ConfigToggle);
customElements.define('file-paths', FilePaths);
customElements.define('history-limit', HistoryLimit);
customElements.define('overlay-size', OverlaySize);
