const status = document.querySelector("#template-script-status");

if (status) {
    status.textContent = "Template script loaded";
}

document.documentElement.dataset.templateScript = "ready";
