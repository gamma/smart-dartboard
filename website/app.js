document.documentElement.classList.add("js");

const header = document.querySelector("[data-header]");
const nav = document.querySelector("[data-nav]");
const navToggle = document.querySelector("[data-nav-toggle]");

function updateHeader() {
  header?.classList.toggle("scrolled", window.scrollY > 24);
}

updateHeader();
window.addEventListener("scroll", updateHeader, { passive: true });

navToggle?.addEventListener("click", () => {
  const open = !nav.classList.contains("open");
  nav.classList.toggle("open", open);
  navToggle.setAttribute("aria-expanded", String(open));
});

nav?.addEventListener("click", event => {
  if (event.target.closest("a")) {
    nav.classList.remove("open");
    navToggle?.setAttribute("aria-expanded", "false");
  }
});

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const revealItems = document.querySelectorAll(".reveal");

if (reducedMotion || !("IntersectionObserver" in window)) {
  revealItems.forEach(item => item.classList.add("visible"));
} else {
  const observer = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      if (!entry.isIntersecting) return;
      entry.target.style.setProperty(
        "--delay",
        `${Number(entry.target.dataset.delay || 0)}ms`,
      );
      entry.target.classList.add("visible");
      observer.unobserve(entry.target);
    });
  }, { threshold: 0.12 });
  revealItems.forEach(item => observer.observe(item));
}

const themeStage = document.querySelector("[data-theme-stage]");
const themeName = themeStage?.querySelector("[data-theme-name]");

themeStage?.addEventListener("click", event => {
  const button = event.target.closest("[data-theme-button]");
  if (!button) return;
  const theme = button.dataset.themeButton;

  themeStage.querySelectorAll("[data-theme-button]").forEach(candidate => {
    const selected = candidate === button;
    candidate.classList.toggle("active", selected);
    candidate.setAttribute("aria-selected", String(selected));
  });
  themeStage.querySelectorAll("[data-theme-image]").forEach(image => {
    image.classList.toggle("active", image.dataset.themeImage === theme);
  });
  if (themeName) {
    themeName.textContent = theme === "neon" ? "CLASSIC NEON" : "PLAYFUL CARTOON";
  }
});

const lightbox = document.querySelector("[data-lightbox]");
const lightboxImage = lightbox?.querySelector("img");

document.addEventListener("click", event => {
  const trigger = event.target.closest("[data-zoom]");
  if (!trigger || !lightbox || !lightboxImage) return;
  const source = trigger.dataset.zoom;
  const thumbnail = trigger.querySelector("img");
  lightboxImage.src = source;
  lightboxImage.alt = thumbnail?.alt || "Vergrößerte Gameplay-Aufnahme";
  lightbox.showModal();
});

lightbox?.addEventListener("click", event => {
  if (event.target === lightbox || event.target.closest("[data-lightbox-close]")) {
    lightbox.close();
  }
});

document.addEventListener("keydown", event => {
  if (event.key === "Escape" && lightbox?.open) {
    lightbox.close();
  }
});

const quickstartCopyButton = document.querySelector("[data-copy-quickstart]");
const quickstartCode = document.querySelector("[data-quickstart-code]");

async function copyQuickstart() {
  if (!quickstartCopyButton || !quickstartCode) return;
  const commands = quickstartCode.textContent.trim();

  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(commands);
    } else {
      const textarea = document.createElement("textarea");
      textarea.value = commands;
      textarea.setAttribute("readonly", "");
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.append(textarea);
      textarea.select();
      const copied = document.execCommand("copy");
      textarea.remove();
      if (!copied) throw new Error("Copy command was rejected");
    }

    quickstartCopyButton.classList.add("copied");
    quickstartCopyButton.innerHTML = '<span aria-hidden="true">✓</span> Kopiert';
    window.setTimeout(() => {
      quickstartCopyButton.classList.remove("copied");
      quickstartCopyButton.innerHTML = '<span aria-hidden="true">□</span> Befehle kopieren';
    }, 2200);
  } catch {
    quickstartCopyButton.textContent = "Bitte manuell markieren";
  }
}

quickstartCopyButton?.addEventListener("click", copyQuickstart);
