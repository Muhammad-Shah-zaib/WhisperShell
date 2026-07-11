export class HistoryLimit extends HTMLElement {
  constructor() {
    super();
    this.innerHTML = `
      <div class="section-container">
        <div class="section-title">History Limit</div>
        <div class="section-desc">Number of previous inputs to save in the logs.</div>
        
        <custom-select id="history-select"></custom-select>
      </div>
    `;

    const options = [
      { value: "0", label: "0 (Do not save history)" },
      { value: "5", label: "5 previous inputs" },
      { value: "10", label: "10 previous inputs" },
      { value: "15", label: "15 previous inputs" },
      { value: "20", label: "20 previous inputs" }
    ];

    const select = this.querySelector('#history-select');
    select.setAttribute('options', JSON.stringify(options));
    select.setAttribute('value', '5'); // default selected
  }
}
