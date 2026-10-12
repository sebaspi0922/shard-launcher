// Pestañas de la galería de estilos.
const tabs = document.querySelectorAll(".tabs button");
for (const tab of tabs) {
  tab.addEventListener("click", () => {
    for (const t of tabs) t.setAttribute("aria-selected", String(t === tab));
    // Una imagen nueva reinicia la animación de entrada.
    const shot = document.getElementById("style-shot");
    const next = shot.cloneNode();
    next.src = tab.dataset.img;
    next.alt = tab.dataset.alt;
    shot.replaceWith(next);
  });
}
