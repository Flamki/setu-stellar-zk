const views = document.querySelectorAll(".view");
const navItems = document.querySelectorAll("[data-view]");
const viewLinks = document.querySelectorAll("[data-view-link]");
const toast = document.getElementById("toast");
const authModal = document.getElementById("auth-modal");
const authMode = document.getElementById("auth-mode");
const authTitle = document.getElementById("auth-title");
const authSubmit = document.getElementById("auth-submit");
const profileName = document.getElementById("profile-name");
const profileEmail = document.getElementById("profile-email");
const activityList = document.getElementById("activity-list");

const activities = [
  {
    title: "Private withdrawal completed",
    detail: "Nullifier stored on Stellar testnet",
    amount: "1,000 XLM",
  },
  {
    title: "Disclosure receipt verified",
    detail: "validProof returned true",
    amount: "OK",
  },
  {
    title: "Tampered receipt rejected",
    detail: "altered public signals returned false",
    amount: "Blocked",
  },
];

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("show");
  window.clearTimeout(showToast.timer);
  showToast.timer = window.setTimeout(() => toast.classList.remove("show"), 2600);
}

function setView(id) {
  views.forEach((view) => view.classList.toggle("active", view.id === id));
  navItems.forEach((item) => item.classList.toggle("active", item.dataset.view === id));
  window.location.hash = id;
}

function openAuth(mode) {
  const isLogin = mode === "login";
  authMode.textContent = isLogin ? "Welcome back" : "Demo access";
  authTitle.textContent = isLogin ? "Log in to Setu" : "Create your Setu account";
  authSubmit.textContent = isLogin ? "Log in" : "Sign up";
  authModal.hidden = false;
}

function closeAuth() {
  authModal.hidden = true;
}

function renderActivities() {
  activityList.innerHTML = activities
    .map(
      (item) => `
        <div class="activity-item">
          <div>
            <strong>${item.title}</strong>
            <span>${item.detail}</span>
          </div>
          <strong>${item.amount}</strong>
        </div>
      `,
    )
    .join("");
}

navItems.forEach((item) => {
  item.addEventListener("click", () => setView(item.dataset.view));
});

viewLinks.forEach((link) => {
  link.addEventListener("click", () => setView(link.dataset.viewLink));
});

document.getElementById("signin-open").addEventListener("click", () => openAuth("login"));
document.getElementById("signup-open").addEventListener("click", () => openAuth("signup"));
document.getElementById("auth-close").addEventListener("click", closeAuth);

authModal.addEventListener("click", (event) => {
  if (event.target === authModal) closeAuth();
});

document.getElementById("auth-form").addEventListener("submit", (event) => {
  event.preventDefault();
  const form = new FormData(event.currentTarget);
  profileName.textContent = form.get("name") || "Setu user";
  profileEmail.textContent = form.get("email") || "user@setu.demo";
  closeAuth();
  showToast("Demo account active");
});

document.getElementById("wallet-button").addEventListener("click", (event) => {
  event.currentTarget.textContent = "Wallet connected";
  showToast("Testnet wallet connected");
});

document.getElementById("send-form").addEventListener("submit", (event) => {
  event.preventDefault();
  const form = new FormData(event.currentTarget);
  const amount = Number(form.get("amount") || 1000).toLocaleString("en-US");
  const asset = form.get("asset") || "XLM";
  document.getElementById("proof-pill").textContent = "Generated";
  document.getElementById("withdraw-step").classList.add("done");
  document.getElementById("receipt-step").classList.add("active");
  document.getElementById("withdraw-copy").textContent = `${amount} ${asset} proof package ready`;
  document.getElementById("proof-json").textContent = JSON.stringify(
    {
      contract: "CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT",
      recipient: form.get("recipient"),
      amount: `${amount} ${asset}`,
      purpose: form.get("purpose"),
      withdrawalProof: "verified",
      disclosureReceipt: "available",
    },
    null,
    2,
  );
  activities.unshift({
    title: "Private transfer generated",
    detail: `${form.get("recipient")} · ${form.get("purpose")}`,
    amount: `${amount} ${asset}`,
  });
  renderActivities();
  showToast("Private transfer generated");
});

document.getElementById("withdraw-button").addEventListener("click", () => {
  showToast("Withdrawal is confirmed on Stellar testnet");
});

document.getElementById("copy-receipt").addEventListener("click", async () => {
  const receipt = {
    receiptId: "SETU-DISC-001",
    amount: "1,000 XLM",
    purpose: "Family support",
    verifyDisclosure: true,
    tamperCheck: false,
    contract: "CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT",
  };

  try {
    await navigator.clipboard.writeText(JSON.stringify(receipt, null, 2));
    showToast("Receipt copied");
  } catch {
    showToast("Receipt ready to copy");
  }
});

renderActivities();

const initialHash = window.location.hash.replace("#", "");
if (initialHash && document.getElementById(initialHash)) {
  setView(initialHash);
}
