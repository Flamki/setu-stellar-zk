// Docs check: keeps the privacy & compliance limitations doc linked, present,
// and honest. Mirrors site/smoke-check.cjs (ok/fail helpers, failure list,
// non-zero exit) so the repo has one recognizable assertion style.
//
// It pins the acceptance criteria for the limitations-doc work:
//   1. The doc exists and is linked from the README near the product story.
//   2. The doc covers what the proof proves, what it does not prove, the
//      anchor/off-ramp mock status, and the legal caveats.
//   3. Claims stay conservative: the doc keeps the prototype markers that make
//      overclaiming impossible to miss.
//   4. The README (Integration Boundaries) and the site landing page stay
//      honest about prototype limits.
//
// Run: node scripts/docs-check.cjs

const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const failures = [];

function ok(message) {
  console.log(`ok   ${message}`);
}

function fail(message) {
  failures.push(message);
  console.error(`FAIL ${message}`);
}

const DOC_REL = "docs/privacy-compliance-limitations.md";
const EVIDENCE_REL = "docs/SUBMISSION-EVIDENCE.md";
const README_REL = "README.md";
const SITE_REL = "site/index.html";

function read(rel) {
  const filePath = path.join(root, rel);
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return fs.readFileSync(filePath, "utf8");
}

// Scope sections the doc must cover (from the issue: what the proof proves,
// what it does not prove, anchor/off-ramp mock status, legal caveats).
const requiredSections = [
  "What the Proof Proves",
  "What the Proof Does Not Prove",
  "Anchor and Off-Ramp Status",
  "Legal Caveats",
];

// Conservative-claim markers. Each is a claim the doc makes about what Setu is
// NOT; if one disappears, the doc is overclaiming and the check fails.
const conservativeMarkers = [
  "not audited",
  "not legal",
  "prototype",
  "stub",
  "trusted setup",
  "testnet",
  "prover-asserted",
];

function checkDocExists() {
  if (read(DOC_REL)) {
    ok(`file exists: ${DOC_REL}`);
  } else {
    fail(`missing file: ${DOC_REL}`);
  }
}

function checkReadmeLinksDoc() {
  const readme = read(README_REL);
  if (!readme) {
    fail(`missing file: ${README_REL}`);
    return;
  }
  if (readme.includes(DOC_REL)) {
    ok(`README links ${DOC_REL}`);
  } else {
    fail(`README does not link ${DOC_REL}`);
  }
}

// Acceptance criterion: "Doc is linked near the product story". The product
// story lives in the README intro, which ends at the first heading
// ("## Live Testnet Deployment"), so the link must appear before it.
function checkLinkNearProductStory() {
  const readme = read(README_REL);
  if (!readme) {
    return;
  }
  const linkAt = readme.indexOf(DOC_REL);
  const storyEndsAt = readme.indexOf("## Live Testnet Deployment");
  if (linkAt === -1) {
    return; // already reported by checkReadmeLinksDoc
  }
  if (storyEndsAt === -1) {
    fail(
      "README lost the 'Live Testnet Deployment' heading; cannot verify the doc link sits near the product story"
    );
    return;
  }
  if (linkAt < storyEndsAt) {
    ok("doc link appears near the product story (before 'Live Testnet Deployment')");
  } else {
    fail("doc link is not near the product story; move it into the README intro");
  }
}

function checkDocSections() {
  const doc = read(DOC_REL);
  if (!doc) {
    return;
  }
  for (const section of requiredSections) {
    if (doc.includes(section)) {
      ok(`doc covers: ${section}`);
    } else {
      fail(`doc missing section: ${section}`);
    }
  }
}

function checkDocConservative() {
  const doc = read(DOC_REL);
  if (!doc) {
    return;
  }
  const lower = doc.toLowerCase();
  for (const marker of conservativeMarkers) {
    if (lower.includes(marker)) {
      ok(`doc stays conservative: mentions '${marker}'`);
    } else {
      fail(`doc may overclaim: no '${marker}' anywhere`);
    }
  }
}

// Acceptance criterion: "README or comments stay honest about prototype
// limits". The Integration Boundaries section must keep its two sharpest
// prototype admissions.
function checkReadmeHonest() {
  const readme = read(README_REL);
  if (!readme) {
    return;
  }
  const lower = readme.toLowerCase();
  for (const marker of ["stub", "not production-secure"]) {
    if (lower.includes(marker)) {
      ok(`README stays honest: '${marker}'`);
    } else {
      fail(`README prototype limit removed: '${marker}'`);
    }
  }
}

// The landing page makes KYC/AML-style claims; if it does, the limitations
// doc must be linked right there so the claim is paired with the caveats.
function checkSiteClaimsHonest() {
  const site = read(SITE_REL);
  if (!site) {
    return;
  }
  const claimsCompliance = /KYC\/AML|Regulatory Standards/i.test(site);
  if (!claimsCompliance) {
    ok("site makes no KYC/AML compliance claim; nothing to pair");
    return;
  }
  if (site.includes("privacy-compliance-limitations")) {
    ok("site KYC/AML claims are paired with the limitations doc link");
  } else {
    fail("site claims KYC/AML compliance without linking the limitations doc");
  }
}

function checkSubmissionEvidenceDoc() {
  const evidence = read(EVIDENCE_REL);
  const readme = read(README_REL);
  if (!evidence) {
    fail(`missing file: ${EVIDENCE_REL}`);
    return;
  }
  ok(`file exists: ${EVIDENCE_REL}`);

  if (readme && readme.includes(EVIDENCE_REL)) {
    ok(`README links ${EVIDENCE_REL}`);
  } else {
    fail(`README does not link ${EVIDENCE_REL}`);
  }

  // Every testnet contract/tx identifier recorded in README must stay in sync
  // with the submission evidence page so links never drift.
  if (readme) {
    const txMatches = readme.match(/stellar\.expert\/explorer\/testnet\/tx\/([0-9a-f]{64})/g) || [];
    for (const txPath of txMatches) {
      if (evidence.includes(txPath)) {
        ok(`evidence doc indexes ${txPath.slice(-16)}`);
      } else {
        fail(`evidence doc missing README testnet tx link: ${txPath}`);
      }
    }
    const contractId = "CDXLQFYQJVDXBZDI5QVYRAM5TGPMZQWS424FCQWYVNGSKSSHPU6XXAXT";
    if (evidence.includes(contractId)) {
      ok("evidence doc includes deployed testnet contract ID");
    } else {
      fail("evidence doc missing deployed testnet contract ID");
    }
  }

  // Every relative repository link in docs/SUBMISSION-EVIDENCE.md must resolve.
  const evidenceDir = path.dirname(path.join(root, EVIDENCE_REL));
  const linkRegex = /\]\((\.\.?\/[^)#]+)\)/g;
  let match;
  while ((match = linkRegex.exec(evidence)) !== null) {
    const relTarget = match[1];
    const resolved = path.resolve(evidenceDir, relTarget);
    if (fs.existsSync(resolved)) {
      ok(`evidence link resolves: ${relTarget}`);
    } else {
      fail(`evidence broken link: ${relTarget}`);
    }
  }

  const lower = evidence.toLowerCase();
  for (const tier of ["live testnet", "local proof", "product mock", "prototype", "not production-secure"]) {
    if (lower.includes(tier)) {
      ok(`evidence doc distinguishes/states '${tier}'`);
    } else {
      fail(`evidence doc missing required boundary marker '${tier}'`);
    }
  }
}

function main() {
  checkDocExists();
  checkReadmeLinksDoc();
  checkLinkNearProductStory();
  checkDocSections();
  checkDocConservative();
  checkReadmeHonest();
  checkSiteClaimsHonest();
  checkSubmissionEvidenceDoc();

  if (failures.length > 0) {
    console.error(`\ndocs check failed with ${failures.length} problem(s):`);
    for (const message of failures) {
      console.error(`- ${message.split("\n")[0]}`);
    }
    process.exit(1);
  }
  console.log("\ndocs check passed");
}

main();
