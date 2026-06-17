const tabs = document.querySelectorAll(".tab");
const panels = document.querySelectorAll(".panel");

tabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    tabs.forEach((item) => item.classList.remove("active"));
    panels.forEach((panel) => panel.classList.remove("active"));
    tab.classList.add("active");
    document.getElementById(tab.dataset.tab).classList.add("active");
  });
});

const canvas = document.getElementById("rail-map");
const ctx = canvas.getContext("2d");
const nodes = [
  { x: 0.12, y: 0.32, r: 4, label: "USD" },
  { x: 0.28, y: 0.22, r: 3, label: "Pool" },
  { x: 0.48, y: 0.38, r: 5, label: "ZK" },
  { x: 0.68, y: 0.28, r: 3, label: "Audit" },
  { x: 0.86, y: 0.48, r: 4, label: "INR" },
  { x: 0.34, y: 0.72, r: 3, label: "Root" },
  { x: 0.62, y: 0.72, r: 3, label: "Receipt" },
];

function resize() {
  const scale = window.devicePixelRatio || 1;
  canvas.width = Math.floor(window.innerWidth * scale);
  canvas.height = Math.floor(window.innerHeight * scale);
  canvas.style.width = `${window.innerWidth}px`;
  canvas.style.height = `${window.innerHeight}px`;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
}

function draw(time) {
  const w = window.innerWidth;
  const h = window.innerHeight;
  ctx.clearRect(0, 0, w, h);
  ctx.lineWidth = 1;

  for (let i = 0; i < nodes.length - 1; i++) {
    const a = nodes[i];
    const b = nodes[i + 1];
    const ax = a.x * w;
    const ay = a.y * h;
    const bx = b.x * w;
    const by = b.y * h;
    const pulse = (Math.sin(time / 700 + i) + 1) / 2;
    ctx.strokeStyle = `rgba(71, 222, 194, ${0.12 + pulse * 0.18})`;
    ctx.beginPath();
    ctx.moveTo(ax, ay);
    ctx.bezierCurveTo((ax + bx) / 2, ay - 55, (ax + bx) / 2, by + 55, bx, by);
    ctx.stroke();
  }

  nodes.forEach((node, index) => {
    const x = node.x * w;
    const y = node.y * h;
    const pulse = (Math.sin(time / 600 + index * 0.7) + 1) / 2;
    ctx.fillStyle = `rgba(71, 222, 194, ${0.18 + pulse * 0.18})`;
    ctx.beginPath();
    ctx.arc(x, y, node.r + 8 + pulse * 8, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = index === 3 ? "#f0c766" : "#47dec2";
    ctx.beginPath();
    ctx.arc(x, y, node.r, 0, Math.PI * 2);
    ctx.fill();
  });

  requestAnimationFrame(draw);
}

resize();
window.addEventListener("resize", resize);
requestAnimationFrame(draw);
