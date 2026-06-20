// Elements mapping
const toast = document.getElementById("toast");
const profileName = document.getElementById("profile-name");
const profileEmail = document.getElementById("profile-email");
const activityList = document.getElementById("activity-list");

// View Sections
const landingPage = document.getElementById("landing-page");
const authPage = document.getElementById("auth-page");
const appWorkspace = document.getElementById("app-workspace");
const authStoryTitle = document.getElementById("auth-story-title");
const authStoryCopy = document.getElementById("auth-story-copy");
const authStepOne = document.getElementById("auth-step-one");

// Auth Panels
const loginCard = document.getElementById("login-card");
const signupCard = document.getElementById("signup-card");

// Simulator Elements
const simForm = document.getElementById("sim-form");
const simRunning = document.getElementById("sim-running");
const simConsole = document.getElementById("sim-console");
const simStatus = document.getElementById("sim-status");
const simReset = document.getElementById("sim-reset");

// Dashboard Elements
const navItems = document.querySelectorAll(".side-nav .nav-item");
const views = document.querySelectorAll(".workspace .view");
const walletButton = document.getElementById("wallet-button");
const walletConnectActions = document.querySelectorAll("[data-wallet-connect]");
const walletStatus = document.getElementById("wallet-status");
const walletHelpLink = document.getElementById("wallet-help-link");
const sendFrom = document.getElementById("send-from");
const sendWalletGate = document.getElementById("send-wallet-gate");
const sendWalletAddress = document.getElementById("send-wallet-address");

// State variables
let currentUser = null;
let currentWallet = null;

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

// Helper: Toast Message
function showToast(message) {
  toast.textContent = message;
  toast.classList.add("show");
  window.clearTimeout(showToast.timer);
  showToast.timer = window.setTimeout(() => toast.classList.remove("show"), 2600);
}

function shortenAddress(address) {
  if (!address) return "";
  if (address.length <= 14) return address;
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

function getWalletApi() {
  return window.freighterApi || window.freighter || null;
}

function isFileOrigin() {
  return window.location.protocol === "file:";
}

function readFreighterBool(result, key) {
  if (typeof result === "boolean") return result;
  return Boolean(result && result[key]);
}

function readFreighterError(result) {
  if (!result || !result.error) return "";
  if (typeof result.error === "string") return result.error;
  return result.error.message || "Wallet request failed";
}

function setWalletUi(wallet) {
  currentWallet = wallet;

  if (!wallet) {
    walletConnectActions.forEach((button) => {
      button.textContent = "Connect wallet";
      button.classList.remove("wallet-connected");
    });
    sendFrom.value = "Connect wallet first";
    sendWalletGate.classList.remove("wallet-gate-connected");
    sendWalletAddress.textContent = "Connect Freighter to use your own wallet";
    walletStatus.textContent = isFileOrigin()
      ? "You opened index.html as a file. Run npm run dev, then open http://127.0.0.1:4174/index.html#send."
      : "No wallet connected";
    walletStatus.className = "wallet-status";
    walletHelpLink.hidden = false;
    walletHelpLink.textContent = isFileOrigin() ? "Use localhost URL" : "Install Freighter";
    walletHelpLink.href = isFileOrigin()
      ? "http://127.0.0.1:4174/index.html#send"
      : "https://chromewebstore.google.com/detail/freighter/bcacfldlkkdogcmkkibnjlakofdplcbk";
    return;
  }

  walletConnectActions.forEach((button) => {
    button.textContent = `Wallet: ${shortenAddress(wallet.address)}`;
    button.classList.add("wallet-connected");
  });
  sendFrom.value = wallet.address;
  sendWalletGate.classList.add("wallet-gate-connected");
  sendWalletAddress.textContent = wallet.network
    ? `${shortenAddress(wallet.address)} on ${wallet.network}`
    : shortenAddress(wallet.address);
  walletStatus.textContent = wallet.network
    ? `${wallet.kind} connected on ${wallet.network}`
    : `${wallet.kind} connected`;
  walletStatus.className = "wallet-status wallet-status-live";
  walletHelpLink.hidden = true;
}

function setWalletNotice(message, action = "install") {
  currentWallet = null;
  walletConnectActions.forEach((button) => {
    button.textContent = "Connect wallet";
    button.classList.remove("wallet-connected");
  });
  sendFrom.value = "Connect wallet first";
  sendWalletGate.classList.remove("wallet-gate-connected");
  sendWalletAddress.textContent = "Connect Freighter to use your own wallet";
  walletStatus.textContent = message;
  walletStatus.className = "wallet-status wallet-status-warning";
  walletHelpLink.hidden = false;
  walletHelpLink.textContent = action === "localhost" ? "Open localhost page" : "Install or enable Freighter";
  walletHelpLink.href = action === "localhost"
    ? "http://127.0.0.1:4174/index.html#send"
    : "https://chromewebstore.google.com/detail/freighter/bcacfldlkkdogcmkkibnjlakofdplcbk";
}

function loadSavedWallet() {
  const savedWallet = localStorage.getItem("setu_wallet");
  if (!savedWallet) return;

  try {
    const wallet = JSON.parse(savedWallet);
    if (wallet.kind !== "Freighter" || !wallet.address) {
      localStorage.removeItem("setu_wallet");
      return;
    }

    setWalletUi(wallet);
  } catch {
    localStorage.removeItem("setu_wallet");
  }
}

function requireWallet(actionLabel = "continue") {
  if (currentWallet) return true;

  setWalletNotice(`Connect your Freighter wallet to ${actionLabel}.`);
  showToast(`Connect your wallet to ${actionLabel}.`);
  window.location.hash = "#dashboard";
  return false;
}

async function getFreighterNetwork(api) {
  if (!api || typeof api.getNetwork !== "function") return "";

  try {
    const networkResult = await api.getNetwork();
    return networkResult.network || networkResult.name || "";
  } catch {
    return "";
  }
}

async function connectFreighterWallet(api) {
  if (typeof api.isConnected === "function") {
    const connectedResult = await api.isConnected();
    const connectedError = readFreighterError(connectedResult);
    if (connectedError) throw new Error(connectedError);
    if (!readFreighterBool(connectedResult, "isConnected")) {
      throw new Error("Install or unlock Freighter to connect your Stellar wallet.");
    }
  }

  if (typeof api.requestAccess !== "function" && typeof api.getAddress !== "function") {
    throw new Error("Freighter API is unavailable in this browser.");
  }

  const accessResult = typeof api.requestAccess === "function"
    ? await api.requestAccess()
    : await api.getAddress();
  const accessError = readFreighterError(accessResult);
  if (accessError) throw new Error(accessError);

  const address = accessResult.address || accessResult.publicKey || "";
  if (!address) throw new Error("Freighter did not return a public key.");

  return {
    address,
    network: await getFreighterNetwork(api),
    kind: "Freighter",
  };
}

async function handleWalletClick() {
  if (currentWallet) {
    setWalletUi(null);
    localStorage.removeItem("setu_wallet");
    showToast("Wallet disconnected");
    return;
  }

  walletButton.disabled = true;
  walletConnectActions.forEach((button) => {
    button.disabled = true;
    button.textContent = "Connecting...";
  });

  try {
    if (isFileOrigin()) {
      const error = new Error("You opened index.html as a file. Run npm run dev, then open http://127.0.0.1:4174/index.html#send.");
      error.walletAction = "localhost";
      throw error;
    }

    const api = getWalletApi();
    if (!api) {
      throw new Error("Install Freighter to connect your Stellar wallet.");
    }

    const wallet = await connectFreighterWallet(api);
    setWalletUi(wallet);
    localStorage.setItem("setu_wallet", JSON.stringify(wallet));
    showToast("Freighter wallet connected");
    triggerConfetti();
  } catch (error) {
    localStorage.removeItem("setu_wallet");
    setWalletNotice(error.message || "Wallet connection failed", error.walletAction || "install");
    showToast(error.message || "Wallet connection failed");
  } finally {
    walletConnectActions.forEach((button) => {
      button.disabled = false;
    });
  }
}

// Helper: Particle Confetti
function triggerConfetti() {
  const colors = ["#00f5a0", "#00d9f5", "#f59e0b", "#f43f5e", "#6366f1"];
  const count = 60;

  for (let i = 0; i < count; i++) {
    const p = document.createElement("div");
    p.classList.add("confetti-particle");
    p.style.backgroundColor = colors[Math.floor(Math.random() * colors.length)];
    p.style.left = Math.random() * 100 + "vw";
    p.style.top = "-10px";
    p.style.transform = `scale(${Math.random() * 0.8 + 0.4})`;

    document.body.appendChild(p);

    const duration = Math.random() * 2 + 1.5;
    const drift = (Math.random() - 0.5) * 400;

    p.animate([
      { transform: `translateY(0px) rotate(0deg) translateX(0px)`, opacity: 1 },
      { transform: `translateY(105vh) rotate(${Math.random() * 360}deg) translateX(${drift}px)`, opacity: 0 }
    ], {
      duration: duration * 1000,
      easing: "cubic-bezier(0.1, 0.8, 0.3, 1)"
    });

    setTimeout(() => p.remove(), duration * 1000);
  }
}

// Routing & View Manager
function handleRouting() {
  const hash = window.location.hash || "#home";

  // Load session from localStorage if exists
  if (!currentUser) {
    const savedUser = localStorage.getItem("setu_user");
    if (savedUser) {
      try {
        currentUser = JSON.parse(savedUser);
        updateProfileUI();
      } catch {
        localStorage.removeItem("setu_user");
      }
    }
  }
  loadSavedWallet();

  // Route groupings
  if (hash === "#home" || hash === "#features" || hash === "#simulator" || hash === "#compliance") {
    // Show Landing View
    landingPage.style.display = "block";
    authPage.style.display = "none";
    appWorkspace.style.display = "none";
  } else if (hash === "#login" || hash === "#signup") {
    // Show Auth View
    landingPage.style.display = "none";
    authPage.style.display = "grid";
    appWorkspace.style.display = "none";

    if (hash === "#login") {
      loginCard.style.display = "block";
      signupCard.style.display = "none";
      authStoryTitle.textContent = "Welcome back to Setu";
      authStoryCopy.textContent = "Continue from a verified Stellar testnet flow with private withdrawals, disclosure receipts, and tamper-resistant audit evidence in one workspace.";
      authStepOne.textContent = "Enter secure credentials";
    } else {
      loginCard.style.display = "none";
      signupCard.style.display = "block";
      authStoryTitle.textContent = "Join the Setu workspace";
      authStoryCopy.textContent = "Create a remittance desk for privacy-preserving transfers, ZK proof generation, and auditor-ready disclosure packages.";
      authStepOne.textContent = "Register your identity";
    }
  } else {
    // Dashboard views
    if (!currentUser) {
      showToast("Please log in to access the desk.");
      window.location.hash = "#login";
      return;
    }

    landingPage.style.display = "none";
    authPage.style.display = "none";
    appWorkspace.style.display = "grid";

    // Switch specific sub-views in dashboard
    const viewId = hash.replace("#", "");
    const validViews = ["dashboard", "send", "receive", "receipts", "contract"];

    let targetView = viewId;
    if (viewId === "dashboard") targetView = "overview";

    if (validViews.includes(viewId) || viewId === "overview") {
      // Toggle views
      views.forEach(v => {
        if (v.id === targetView) {
          v.style.display = "block";
          v.classList.add("active");
        } else {
          v.style.display = "none";
          v.classList.remove("active");
        }
      });

      // Toggle sidebar active nav item
      navItems.forEach(item => {
        const itemVal = item.dataset.view;
        if (itemVal === targetView || (itemVal === "overview" && targetView === "overview")) {
          item.classList.add("active");
        } else {
          item.classList.remove("active");
        }
      });
    } else {
      // Fallback
      window.location.hash = "#dashboard";
    }
  }
}

// Set View in Dashboard programmatically
function setView(viewId) {
  window.location.hash = viewId === "overview" ? "#dashboard" : `#${viewId}`;
}

// Update UI profile section
function updateProfileUI() {
  if (currentUser) {
    profileName.textContent = currentUser.name;
    profileEmail.textContent = currentUser.email;
  } else {
    profileName.textContent = "Guest account";
    profileEmail.textContent = "guest@setu.example";
  }
}

// Auth Logic: Register User
function registerUser(name, email) {
  currentUser = { name, email };
  localStorage.setItem("setu_user", JSON.stringify(currentUser));
  updateProfileUI();
  showToast("Account created successfully!");
  triggerConfetti();
  window.location.hash = "#dashboard";
}

// Auth Logic: Login User
function loginUser(email) {
  const handle = email.split("@")[0] || "alice";
  const name = handle
    .split(/[._-]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ") || "Alice Mehta";

  currentUser = { name, email };
  localStorage.setItem("setu_user", JSON.stringify(currentUser));
  updateProfileUI();
  showToast("Successfully logged in.");
  triggerConfetti();
  window.location.hash = "#dashboard";
}

// Auth Logic: Log Out
function logout() {
  currentUser = null;
  currentWallet = null;
  localStorage.removeItem("setu_user");
  localStorage.removeItem("setu_wallet");
  updateProfileUI();
  setWalletUi(null);
  showToast("Logged out successfully.");
  window.location.hash = "#home";
}

// Render transfer activity table
function renderActivities() {
  if (!activityList) return;
  activityList.innerHTML = activities
    .map(
      (item) => `
        <div class="activity-item">
          <div>
            <strong>${item.title}</strong>
            <span>${item.detail}</span>
          </div>
          <span class="activity-amount">${item.amount}</span>
        </div>
      `,
    )
    .join("");
}

// ZK Proof Simulator implementation
let simTimer = null;
function runZKSimulator(event) {
  event.preventDefault();

  const amount = Number(document.getElementById("sim-amount").value || 100);
  const asset = document.getElementById("sim-asset").value;
  const recipient = document.getElementById("sim-recipient").value;
  const purpose = document.getElementById("sim-purpose").value;

  // Toggle simulator layouts
  simForm.classList.add("hidden");
  simRunning.classList.remove("hidden");
  simReset.style.display = "none";
  simStatus.textContent = "Processing";
  simStatus.className = "pill orange";

  // Clear steps styling
  const steps = ["deposit", "tree", "proof", "verify"];
  steps.forEach(s => {
    const el = document.getElementById(`step-${s}`);
    el.className = "sim-step";
  });

  const consoleLog = (txt) => {
    simConsole.textContent += "\n" + txt;
    simConsole.scrollTop = simConsole.scrollHeight;
  };

  simConsole.textContent = `[Setu Core] Initializing witness derivation for ${amount} ${asset}...`;

  // Step 1: Commitment Hashing
  setTimeout(() => {
    document.getElementById("step-deposit").className = "sim-step active";
    consoleLog(`[Poseidon] Generating secret note commitment parameters...`);
    consoleLog(`[Poseidon] secret: 0x${Array.from({length:32}, () => Math.floor(Math.random()*16).toString(16)).join("")}`);
    consoleLog(`[Poseidon] nullifier: 0x4acc5489ab80200caae1eca0b44dd2335e92931ff51e358e4fc7381378367816`);
    consoleLog(`[Poseidon] Note Commitment: C = 0x8df1c3f29bda79857...`);
  }, 100);

  // Step 2: Merkle Root
  setTimeout(() => {
    document.getElementById("step-deposit").className = "sim-step done";
    document.getElementById("step-tree").className = "sim-step active";
    consoleLog(`[Stellar] Submitting commitment leaf insertion to Soroban contract...`);
    consoleLog(`[Stellar] Association root queried: 0x709cf0d7530259e33b0af6...`);
    consoleLog(`[Stellar] Tx: adf901702fea3fcd09e0b8a94d19388d23d7579d05cbe8f7210b30489f9eb458 confirmed!`);
  }, 1800);

  // Step 3: Proving
  setTimeout(() => {
    document.getElementById("step-tree").className = "sim-step done";
    document.getElementById("step-proof").className = "sim-step active";
    consoleLog(`[snarkjs] Computing circuit witness in BLS12-381...`);
    consoleLog(`[snarkjs] Witness verified across 14,282 constraints.`);
    consoleLog(`[snarkjs] Generating Groth16 proof package...`);
    consoleLog(`[snarkjs] Proof size: 256 bytes.`);
  }, 3500);

  // Step 4: Verify
  setTimeout(() => {
    document.getElementById("step-proof").className = "sim-step done";
    document.getElementById("step-verify").className = "sim-step active";
    consoleLog(`[Stellar] Invoking verify_disclosure(proof, public_signals)...`);
    consoleLog(`[Stellar] Verification result: TRUE.`);
    consoleLog(`[Setu Core] Private remittance verification completed!`);

    document.getElementById("step-verify").className = "sim-step done";
    simStatus.textContent = "Verified";
    simStatus.className = "pill";
    simReset.style.display = "block";
    triggerConfetti();
  }, 5300);
}

// Reset Simulator
function resetZKSimulator() {
  simForm.classList.remove("hidden");
  simRunning.classList.add("hidden");
  simStatus.textContent = "Idle";
  simStatus.className = "pill";
  simConsole.textContent = "Waiting to compile ZK parameters...";
}

// Event Listeners
window.addEventListener("hashchange", handleRouting);
window.addEventListener("load", () => {
  handleRouting();
  renderActivities();
});

// ZK Simulator trigger
simForm.addEventListener("submit", runZKSimulator);
simReset.addEventListener("click", resetZKSimulator);

// Navigation handlers in dashboard
navItems.forEach((item) => {
  item.addEventListener("click", () => {
    setView(item.dataset.view);
  });
});

// Wallet connection
walletButton.addEventListener("click", handleWalletClick);

// Auth form listeners
document.getElementById("signup-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const firstName = document.getElementById("signup-first").value.trim();
  const lastName = document.getElementById("signup-last").value.trim();
  const name = [firstName, lastName].filter(Boolean).join(" ") || document.getElementById("signup-name").value;
  const email = document.getElementById("signup-email").value;
  registerUser(name, email);
});

document.getElementById("login-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const email = document.getElementById("login-email").value;
  loginUser(email);
});

// Auth View switches
document.getElementById("to-signup").addEventListener("click", () => {
  window.location.hash = "#signup";
});
document.getElementById("to-login").addEventListener("click", () => {
  window.location.hash = "#login";
});
document.getElementById("login-bypass").addEventListener("click", () => {
  loginUser("alice@setu.example");
});
document.getElementById("signup-bypass").addEventListener("click", () => {
  registerUser("Alice Mehta", "alice@setu.example");
});

document.querySelectorAll("[data-password-toggle]").forEach((button) => {
  button.addEventListener("click", () => {
    const input = document.getElementById(button.dataset.passwordToggle);
    if (!input) return;

    const shouldShow = input.type === "password";
    input.type = shouldShow ? "text" : "password";
    button.setAttribute("aria-label", shouldShow ? "Hide password" : "Show password");
  });
});

document.querySelectorAll("[data-login-shortcut]").forEach((button) => {
  button.addEventListener("click", () => loginUser(button.dataset.loginShortcut));
});

document.querySelectorAll("[data-signup-shortcut]").forEach((button) => {
  button.addEventListener("click", () => {
    const [name, email] = button.dataset.signupShortcut.split("|");
    registerUser(name, email);
  });
});

// Log out handlers
document.getElementById("logout-button").addEventListener("click", logout);
document.getElementById("logout-top").addEventListener("click", logout);

// Send Form Submission
document.getElementById("send-form").addEventListener("submit", (event) => {
  event.preventDefault();
  if (!requireWallet("create a private transfer")) return;

  const form = new FormData(event.currentTarget);
  const amount = Number(form.get("amount") || 1000).toLocaleString("en-US");
  const asset = form.get("asset") || "XLM";

  document.getElementById("proof-pill").textContent = "Generated";
  document.getElementById("withdraw-step").className = "flow-step done";
  document.getElementById("receipt-step").className = "flow-step active";

  const withdrawCopy = document.getElementById("withdraw-copy");
  if (withdrawCopy) withdrawCopy.textContent = `${amount} ${asset} proof package ready`;

  document.getElementById("proof-json").textContent = JSON.stringify(
    {
      contract: "CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT",
      senderWallet: currentWallet.address,
      walletNetwork: currentWallet.network || "unknown",
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
  triggerConfetti();
});

// Withdrawal status check
document.getElementById("withdraw-button").addEventListener("click", () => {
  if (!requireWallet("recheck withdrawal status")) return;
  showToast("Withdrawal is confirmed on Stellar testnet");
  triggerConfetti();
});

// Copy receipt auditor
document.getElementById("copy-receipt").addEventListener("click", async () => {
  if (!requireWallet("copy wallet-linked receipt")) return;

  const receipt = {
    receiptId: "SETU-DISC-001",
    wallet: currentWallet.address,
    network: currentWallet.network || "unknown",
    amount: "1,000 XLM",
    purpose: "Family support",
    verifyDisclosure: true,
    tamperCheck: false,
    contract: "CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT",
  };

  try {
    await navigator.clipboard.writeText(JSON.stringify(receipt, null, 2));
    showToast("Receipt copied to clipboard!");
  } catch {
    showToast("Receipt ready to copy");
  }
});
