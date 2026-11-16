// A docs page's only script: the narrow-screen menu. It loads nothing else, so a docs page never fetches the engine.
const sidebar = document.getElementById("sidebar");
const toggle = document.querySelector(".menu-toggle");
toggle?.addEventListener("click", () => {
  const open = sidebar.classList.toggle("open");
  toggle.setAttribute("aria-expanded", String(open));
});
