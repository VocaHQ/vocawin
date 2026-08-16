document.documentElement.classList.add("js");

const menuToggle = document.querySelector("[data-menu-toggle]");
const navigation = document.querySelector("[data-navigation]");
const menuLabel = menuToggle?.querySelector(".sr-only");

if (menuToggle && navigation) {
  const closeNavigation = ({ returnFocus = false } = {}) => {
    menuToggle.setAttribute("aria-expanded", "false");
    if (menuLabel) menuLabel.textContent = "Open navigation";
    navigation.classList.remove("is-open");
    if (returnFocus) menuToggle.focus();
  };

  menuToggle.addEventListener("click", () => {
    const isOpen = menuToggle.getAttribute("aria-expanded") === "true";
    if (isOpen) {
      closeNavigation();
    } else {
      menuToggle.setAttribute("aria-expanded", "true");
      if (menuLabel) menuLabel.textContent = "Close navigation";
      navigation.classList.add("is-open");
    }
  });

  navigation.addEventListener("click", (event) => {
    if (event.target instanceof HTMLAnchorElement) {
      closeNavigation();
    }
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && menuToggle.getAttribute("aria-expanded") === "true") {
      closeNavigation({ returnFocus: true });
    }
  });

  window.addEventListener("resize", () => {
    if (window.innerWidth > 920 && menuToggle.getAttribute("aria-expanded") === "true") {
      closeNavigation();
    }
  });
}

const timeNode = document.querySelector("[data-local-time]");
const yearNodes = document.querySelectorAll("[data-current-year]");

function updateClock() {
  const now = new Date();
  if (timeNode) {
    timeNode.textContent = new Intl.DateTimeFormat(undefined, {
      hour: "numeric",
      minute: "2-digit",
    }).format(now);
  }
  yearNodes.forEach((node) => {
    node.textContent = String(now.getFullYear());
  });
}

updateClock();
window.setInterval(updateClock, 30_000);

const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const revealNodes = document.querySelectorAll(".reveal");

if (reduceMotion.matches || !("IntersectionObserver" in window)) {
  revealNodes.forEach((node) => node.classList.add("is-visible"));
} else {
  const revealObserver = new IntersectionObserver(
    (entries, observer) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add("is-visible");
          observer.unobserve(entry.target);
        }
      });
    },
    { threshold: 0.14 },
  );

  revealNodes.forEach((node) => revealObserver.observe(node));

  window.setTimeout(() => {
    revealNodes.forEach((node) => node.classList.add("is-visible"));
  }, 900);
}

const demoCopy = document.querySelector("[data-demo-copy]");
const phrases = [
  "Ship the notes after lunch and move the review to Thursday.",
  "Leave the draft in the shared folder when you are done.",
  "Ping the room if the build is still running at six.",
];

if (demoCopy && !reduceMotion.matches) {
  let phraseIndex = 0;
  window.setInterval(() => {
    phraseIndex = (phraseIndex + 1) % phrases.length;
    demoCopy.classList.add("is-changing");
    window.setTimeout(() => {
      demoCopy.textContent = phrases[phraseIndex];
      demoCopy.classList.remove("is-changing");
    }, 180);
  }, 4_600);
}

const faqItems = document.querySelectorAll(".faq-list details");
faqItems.forEach((detail) => {
  detail.addEventListener("toggle", () => {
    if (!detail.open) return;
    faqItems.forEach((other) => {
      if (other !== detail) other.open = false;
    });
  });
});
