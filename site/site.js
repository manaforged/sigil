const cards = [...document.querySelectorAll(".example")];
const filterButtons = [...document.querySelectorAll("[data-filter]")];
const visibleCount = document.querySelector("[data-visible-count]");
const emptyResults = document.querySelector("[data-empty-results]");

function filterExamples(category) {
  let count = 0;
  for (const card of cards) {
    const visible = category === "all" || card.dataset.category === category;
    card.hidden = !visible;
    if (visible) count += 1;
  }
  for (const button of filterButtons) {
    button.setAttribute("aria-pressed", String(button.dataset.filter === category));
  }
  visibleCount.textContent = String(count);
  emptyResults.hidden = count !== 0;
}

for (const button of filterButtons) {
  button.addEventListener("click", () => filterExamples(button.dataset.filter));
}

for (const button of document.querySelectorAll("[data-copy]")) {
  button.addEventListener("click", async () => {
    const label = button.querySelector("[data-copy-label]");
    button.removeAttribute("data-state");
    button.setAttribute("aria-busy", "true");
    try {
      await navigator.clipboard.writeText(button.dataset.copy);
      button.dataset.state = "success";
      label.textContent = "Copied";
    } catch {
      button.dataset.state = "error";
      label.textContent = "Copy failed";
    } finally {
      button.removeAttribute("aria-busy");
      window.setTimeout(() => {
        button.removeAttribute("data-state");
        label.textContent = "Copy command";
      }, 2500);
    }
  });
}

const dialog = document.querySelector("[data-command-dialog]");
const commandInput = document.querySelector("[data-command-input]");
const commandResults = document.querySelector("[data-command-results]");
let activeResult = 0;

const commands = cards.map((card) => ({
  category: card.querySelector(".example__kind").textContent,
  href: `#${card.id}`,
  label: card.querySelector("h3").textContent,
  search: `${card.dataset.search} ${card.textContent}`.toLowerCase(),
}));

function commandLinks() {
  return [...commandResults.querySelectorAll("a")];
}

function selectResult(index) {
  const links = commandLinks();
  if (links.length === 0) return;
  activeResult = (index + links.length) % links.length;
  links.forEach((link, position) => {
    link.setAttribute("aria-selected", String(position === activeResult));
  });
  links[activeResult].scrollIntoView({ block: "nearest" });
}

function renderCommands(query = "") {
  const needle = query.trim().toLowerCase();
  const matches = commands.filter((command) => !needle || command.search.includes(needle));
  commandResults.replaceChildren();
  activeResult = 0;

  for (const [index, command] of matches.entries()) {
    const link = document.createElement("a");
    const label = document.createElement("span");
    const category = document.createElement("small");
    link.className = "command-result";
    link.href = command.href;
    link.setAttribute("role", "option");
    link.setAttribute("aria-selected", String(index === 0));
    label.textContent = command.label;
    category.textContent = command.category;
    link.append(label, category);
    link.addEventListener("click", () => {
      filterExamples("all");
      dialog.close();
    });
    commandResults.append(link);
  }

  if (matches.length === 0) {
    const message = document.createElement("p");
    message.className = "empty-results";
    message.textContent = "No examples match that term.";
    commandResults.append(message);
  }
}

function openCommandDialog() {
  commandInput.value = "";
  renderCommands();
  dialog.showModal();
  commandInput.focus();
}

for (const button of document.querySelectorAll("[data-open-search]")) {
  button.addEventListener("click", openCommandDialog);
}

commandInput.addEventListener("input", () => renderCommands(commandInput.value));

dialog.addEventListener("click", (event) => {
  if (event.target === dialog) dialog.close();
});

dialog.addEventListener("keydown", (event) => {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    selectResult(activeResult + 1);
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    selectResult(activeResult - 1);
  }
  if (event.key === "Enter" && event.target === commandInput) {
    const link = commandLinks()[activeResult];
    if (link) link.click();
  }
});

document.addEventListener("keydown", (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    dialog.open ? dialog.close() : openCommandDialog();
  }
});
