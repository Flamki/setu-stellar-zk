const fs = require("fs");
const path = require("path");

const config = {
  url: process.env.SETU_SUPABASE_URL || process.env.SUPABASE_URL || "",
  anonKey:
    process.env.SETU_SUPABASE_ANON_KEY ||
    process.env.SUPABASE_ANON_KEY ||
    process.env.SUPABASE_PUBLISHABLE_KEY ||
    "",
};

const enabled = Boolean(config.url && config.anonKey);
const payload = {
  enabled,
  url: config.url,
  anonKey: config.anonKey,
};

const body = [
  "window.SETU_SUPABASE_CONFIG = ",
  JSON.stringify(payload, null, 2),
  ";",
  "",
].join("");

fs.writeFileSync(path.join(__dirname, "auth-config.js"), body, "utf8");
console.log(
  enabled
    ? "Setu auth config generated from Supabase environment."
    : "Setu auth config generated in disabled mode. Set SETU_SUPABASE_URL and SETU_SUPABASE_ANON_KEY to enable auth.",
);
