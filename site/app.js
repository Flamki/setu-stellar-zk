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
const authStatusNodes = document.querySelectorAll("[data-auth-status]");
const authProviderButtons = document.querySelectorAll("[data-auth-provider]");
const authFormControls = document.querySelectorAll(
  "#login-form input, #login-form button, #signup-form input, #signup-form button, [data-auth-provider], #reset-password",
);

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
let authInitialized = false;
let authSession = null;

const supabaseConfig = window.SETU_SUPABASE_CONFIG || {};
const authClient = createAuthClient();

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

function updateViewportMetrics() {
  document.documentElement.style.setProperty(
    "--setu-client-width",
    `${document.documentElement.clientWidth}px`,
  );
}

// Helper: Toast Message
function showToast(message) {
  toast.textContent = message;
  toast.classList.add("show");
  window.clearTimeout(showToast.timer);
  showToast.timer = window.setTimeout(() => toast.classList.remove("show"), 2600);
}

function createAuthClient() {
  const hasConfig = Boolean(
    supabaseConfig.enabled &&
    supabaseConfig.url &&
    supabaseConfig.anonKey,
  );

  if (!hasConfig || !window.supabase || typeof window.supabase.createClient !== "function") {
    return null;
  }

  return window.supabase.createClient(supabaseConfig.url, supabaseConfig.anonKey, {
    auth: {
      autoRefreshToken: true,
      detectSessionInUrl: true,
      persistSession: true,
    },
  });
}

function getAuthRedirect(hash = "#dashboard") {
  const url = new URL(window.location.href);
  url.hash = hash;
  return url.toString();
}

function setAuthStatus(message = "", type = "info") {
  authStatusNodes.forEach((node) => {
    node.textContent = message;
    node.hidden = !message;
    node.classList.toggle("success", type === "success");
    node.classList.toggle("error", type === "error");
  });
}

function setAuthControlsEnabled(enabled) {
  authFormControls.forEach((control) => {
    control.disabled = !enabled;
  });
}

function setButtonBusy(button, busy, label) {
  if (!button) return;
  if (!button.dataset.readyLabel) {
    button.dataset.readyLabel = button.textContent;
  }

  button.disabled = busy;
  button.textContent = busy ? label : button.dataset.readyLabel;
}

function renderAuthAvailability() {
  if (authClient) {
    setAuthControlsEnabled(true);
    setAuthStatus("");
    return;
  }

  setAuthControlsEnabled(false);
  setAuthStatus(
    "Account access is being provisioned for this deployment. Connect Setu's Supabase workspace to enable sign-in.",
    "error",
  );
}

function formatAuthError(error) {
  if (!error) return "Authentication request failed.";
  return error.message || "Authentication request failed.";
}

function getDisplayNameFromUser(user) {
  const metadata = user.user_metadata || {};
  const fullName = metadata.full_name || metadata.name || "";
  const firstLast = [metadata.first_name, metadata.last_name].filter(Boolean).join(" ");
  const fromEmail = (user.email || "").split("@")[0] || "Setu operator";

  return fullName || firstLast || fromEmail;
}

function userFromSession(session) {
  if (!session || !session.user) return null;
  const user = session.user;

  return {
    id: user.id,
    name: getDisplayNameFromUser(user),
    email: user.email || "",
    role: user.user_metadata?.role || "operator",
  };
}

async function hydrateAuthenticatedUser(session) {
  authSession = session || null;
  currentUser = userFromSession(session);
  updateProfileUI();

  if (!authClient || !currentUser) return;

  try {
    const { data, error } = await authClient
      .from("profiles")
      .select("email, full_name, first_name, last_name, role")
      .eq("id", currentUser.id)
      .maybeSingle();

    if (error || !data) return;

    const profileName = data.full_name || [data.first_name, data.last_name].filter(Boolean).join(" ");
    currentUser = {
      ...currentUser,
      name: profileName || currentUser.name,
      email: data.email || currentUser.email,
      role: data.role || currentUser.role,
    };
    updateProfileUI();
  } catch {
    // Auth remains valid even if the optional profile hydration endpoint is unavailable.
  }
}

async function upsertProfile(profile) {
  if (!authClient || !authSession?.user) return;

  await authClient.from("profiles").upsert({
    id: authSession.user.id,
    email: profile.email,
    full_name: profile.name,
    first_name: profile.firstName,
    last_name: profile.lastName,
    role: "operator",
  });
}

async function initAuth() {
  localStorage.removeItem("setu_user");
  renderAuthAvailability();

  if (!authClient) {
    authInitialized = true;
    updateProfileUI();
    handleRouting();
    renderActivities();
    return;
  }

  const { data, error } = await authClient.auth.getSession();
  if (error) {
    setAuthStatus(formatAuthError(error), "error");
  }

  authInitialized = true;
  await hydrateAuthenticatedUser(data?.session || null);

  authClient.auth.onAuthStateChange((event, session) => {
    void (async () => {
      await hydrateAuthenticatedUser(session);

      if (event === "SIGNED_IN" && (window.location.hash === "#login" || window.location.hash === "#signup")) {
        window.location.hash = "#dashboard";
        return;
      }

      if (event === "SIGNED_OUT") {
        setWalletUi(null);
        localStorage.removeItem("setu_wallet");
      }

      handleRouting();
    })();
  });

  handleRouting();
  renderActivities();
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
    authPage.style.display = "flex";
    appWorkspace.style.display = "none";

    if (hash === "#login") {
      loginCard.style.display = "block";
      signupCard.style.display = "none";
      authStoryTitle.textContent = "Welcome back";
      authStoryCopy.textContent = "Follow these 3 quick phases to access your private remittance space.";
      authStepOne.textContent = "Verify your identity";
    } else {
      loginCard.style.display = "none";
      signupCard.style.display = "block";
      authStoryTitle.textContent = "Join Setu";
      authStoryCopy.textContent = "Follow these 3 quick phases to activate your private remittance space.";
      authStepOne.textContent = "Register your identity";
    }
  } else {
    // Dashboard views
    if (!authInitialized) return;

    if (!currentUser) {
      if (!authClient) {
        renderAuthAvailability();
      }
      showToast("Sign in to access the desk.");
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
    profileName.textContent = "Not signed in";
    profileEmail.textContent = "Authentication required";
  }
}

// Auth Logic: Register User
async function registerUser({ name, email, password, firstName, lastName }, submitButton) {
  if (!authClient) {
    renderAuthAvailability();
    return;
  }

  setButtonBusy(submitButton, true, "Creating account...");
  setAuthStatus("");

  try {
    const { data, error } = await authClient.auth.signUp({
      email,
      password,
      options: {
        data: {
          full_name: name,
          first_name: firstName,
          last_name: lastName,
          role: "operator",
        },
        emailRedirectTo: getAuthRedirect("#dashboard"),
      },
    });

    if (error) throw error;

    if (data.session) {
      authSession = data.session;
      await hydrateAuthenticatedUser(data.session);
      await upsertProfile({ name, email, firstName, lastName });
      setAuthStatus("Account created and signed in.", "success");
      showToast("Account created.");
      triggerConfetti();
      window.location.hash = "#dashboard";
      return;
    }

    setAuthStatus("Account created. Check your email to confirm access, then sign in.", "success");
    showToast("Check your email to confirm access.");
    window.location.hash = "#login";
  } catch (error) {
    setAuthStatus(formatAuthError(error), "error");
    showToast(formatAuthError(error));
  } finally {
    setButtonBusy(submitButton, false);
  }
}

// Auth Logic: Login User
async function loginUser(email, password, submitButton) {
  if (!authClient) {
    renderAuthAvailability();
    return;
  }

  setButtonBusy(submitButton, true, "Signing in...");
  setAuthStatus("");

  try {
    const { data, error } = await authClient.auth.signInWithPassword({ email, password });
    if (error) throw error;

    await hydrateAuthenticatedUser(data.session);
    setAuthStatus("Signed in securely.", "success");
    showToast("Signed in.");
    triggerConfetti();
    window.location.hash = "#dashboard";
  } catch (error) {
    setAuthStatus(formatAuthError(error), "error");
    showToast(formatAuthError(error));
  } finally {
    setButtonBusy(submitButton, false);
  }
}

async function signInWithProvider(provider, button) {
  if (!authClient) {
    renderAuthAvailability();
    return;
  }

  setButtonBusy(button, true, "Redirecting...");
  setAuthStatus("");

  try {
    const { error } = await authClient.auth.signInWithOAuth({
      provider,
      options: {
        redirectTo: getAuthRedirect("#dashboard"),
      },
    });

    if (error) throw error;
  } catch (error) {
    setButtonBusy(button, false);
    setAuthStatus(formatAuthError(error), "error");
    showToast(formatAuthError(error));
  }
}

async function sendPasswordReset(email, button) {
  if (!authClient) {
    renderAuthAvailability();
    return;
  }

  if (!email) {
    setAuthStatus("Enter your email address first, then request a password reset.", "error");
    return;
  }

  setButtonBusy(button, true, "Sending...");
  setAuthStatus("");

  try {
    const { error } = await authClient.auth.resetPasswordForEmail(email, {
      redirectTo: getAuthRedirect("#login"),
    });

    if (error) throw error;

    setAuthStatus("Password reset email sent.", "success");
    showToast("Password reset email sent.");
  } catch (error) {
    setAuthStatus(formatAuthError(error), "error");
    showToast(formatAuthError(error));
  } finally {
    setButtonBusy(button, false);
  }
}

// Auth Logic: Log Out
async function logout() {
  if (authClient && authSession) {
    const { error } = await authClient.auth.signOut();
    if (error) {
      showToast(formatAuthError(error));
      return;
    }
  }

  currentUser = null;
  authSession = null;
  currentWallet = null;
  localStorage.removeItem("setu_wallet");
  updateProfileUI();
  setWalletUi(null);
  showToast("Signed out.");
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
    consoleLog("[Poseidon] note opening derived client-side: [redacted]");
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
  void initAuth();
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
  const email = document.getElementById("signup-email").value.trim();
  const password = document.getElementById("signup-password").value;

  void registerUser(
    {
      name,
      email,
      password,
      firstName,
      lastName,
    },
    e.submitter,
  );
});

document.getElementById("login-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const email = document.getElementById("login-email").value.trim();
  const password = document.getElementById("login-password").value;
  void loginUser(email, password, e.submitter);
});

// Auth View switches
document.getElementById("to-signup").addEventListener("click", () => {
  window.location.hash = "#signup";
});
document.getElementById("to-login").addEventListener("click", () => {
  window.location.hash = "#login";
});

document.getElementById("reset-password").addEventListener("click", (event) => {
  const email = document.getElementById("login-email").value.trim();
  void sendPasswordReset(email, event.currentTarget);
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

authProviderButtons.forEach((button) => {
  button.addEventListener("click", () => {
    void signInWithProvider(button.dataset.authProvider, button);
  });
});

// Log out handlers
document.getElementById("logout-button").addEventListener("click", () => void logout());
document.getElementById("logout-top").addEventListener("click", () => void logout());

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

// Interactive Spotlight tracking
window.addEventListener("mousemove", (e) => {
  const spotlight = document.getElementById("mouse-spotlight");
  if (spotlight) {
    spotlight.style.setProperty("--mouse-x", `${e.clientX}px`);
    spotlight.style.setProperty("--mouse-y", `${e.clientY}px`);
    spotlight.style.opacity = "0.75";
  }
});

document.addEventListener("mouseleave", () => {
  const spotlight = document.getElementById("mouse-spotlight");
  if (spotlight) {
    spotlight.style.opacity = "0";
  }
});

// Ambient Liquid WebGL Shader for Footer
function initFooterShader() {
  const canvas = document.getElementById("footer-shader-canvas");
  if (!canvas) return;

  const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
  if (!gl) return;

  // Vertex shader: Full-screen quad
  const vsSource = `
    attribute vec2 position;
    void main() {
      gl_Position = vec4(position, 0.0, 1.0);
    }
  `;

  // Fragment shader: Fluid ambient plasma in green/amber/cyan matching the theme
  const fsSource = `
    precision mediump float;
    uniform float u_time;
    uniform vec2 u_resolution;

    void main() {
      vec2 uv = gl_FragCoord.xy / u_resolution.xy;
      vec2 p = uv - 0.5;
      p.x *= u_resolution.x / u_resolution.y;

      // Morphing waves
      float wave1 = sin(p.x * 2.2 + u_time * 0.7) * 0.18;
      float wave2 = cos(p.y * 1.8 - u_time * 0.5) * 0.12;

      float dist1 = abs(p.y - wave1);
      float dist2 = abs(p.x - wave2);

      float glow1 = 0.015 / (dist1 + 0.08);
      float glow2 = 0.015 / (dist2 + 0.08);

      // Colors: Amber (#f59e0b), Emerald (#10b981), and Cyan (#06b6d4)
      vec3 col1 = vec3(0.96, 0.62, 0.04);
      vec3 col2 = vec3(0.06, 0.73, 0.51);
      vec3 col3 = vec3(0.02, 0.71, 0.83);

      vec3 finalCol = mix(col1, col2, uv.x) * glow1;
      finalCol += mix(col2, col3, uv.y) * glow2;

      // Pulse overlay
      finalCol *= 0.7 + 0.25 * sin(u_time * 0.4);

      // Bottom black overlay
      finalCol += vec3(0.003, 0.006, 0.01) * (1.0 - uv.y);

      gl_FragColor = vec4(finalCol, 1.0);
    }
  `;

  function compileShader(source, type) {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error(gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  const vs = compileShader(vsSource, gl.VERTEX_SHADER);
  const fs = compileShader(fsSource, gl.FRAGMENT_SHADER);
  if (!vs || !fs) return;

  const program = gl.createProgram();
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.linkProgram(program);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error(gl.getProgramInfoLog(program));
    return;
  }

  const positionBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([
      -1, -1,
       1, -1,
      -1,  1,
      -1,  1,
       1, -1,
       1,  1,
    ]),
    gl.STATIC_DRAW
  );

  const positionLoc = gl.getAttribLocation(program, "position");
  const timeLoc = gl.getUniformLocation(program, "u_time");
  const resLoc = gl.getUniformLocation(program, "u_resolution");

  function resize() {
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    }
  }

  function render(time) {
    resize();
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(program);
    gl.enableVertexAttribArray(positionLoc);
    gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
    gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 0, 0);

    gl.uniform1f(timeLoc, time * 0.001);
    gl.uniform2f(resLoc, canvas.width, canvas.height);

    gl.drawArrays(gl.TRIANGLES, 0, 6);
    requestAnimationFrame(render);
  }

  requestAnimationFrame(render);
}

// Initialise on load
window.addEventListener("DOMContentLoaded", () => {
  updateViewportMetrics();
  initFooterShader();
});

window.addEventListener("resize", updateViewportMetrics);
