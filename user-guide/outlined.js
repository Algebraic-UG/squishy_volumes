customElements.define(
  "outlined-video",
  class extends HTMLElement {
    connectedCallback() {
      this.innerHTML = `
        <video
          src="${this.getAttribute("src")}"
          controls
          autoplay
          muted
          loop
          playsinline
          style="
            margin: 20px auto;
            box-shadow: 0 0 10px 10px rgba(69, 63, 186, 0.65);
          "
        ></video>
      `;
    }
  }
);

customElements.define(
  "outlined-img",
  class extends HTMLElement {
    connectedCallback() {
      this.innerHTML = `
        <img
          src="${this.getAttribute("src")}"
          alt="${this.getAttribute("alt")}"
          style="
            margin: 20px auto;
            box-shadow: 0 0 10px 10px rgba(69, 63, 186, 0.65);
          "
        >
      `;
    }
  }
);
