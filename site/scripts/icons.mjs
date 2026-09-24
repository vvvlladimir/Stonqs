// Renders the site's icons from the app's own app-icon.svg, cut to the macOS squircle so a browser
// tab shows the same shape as the Dock. The apple-touch icon stays square: iOS masks it itself and
// would paint transparent corners black.
import sharp from "sharp";

const SRC = "../app/app-icon.svg";

// Superellipse n=5, the same curve app/scripts/make-icons.sh uses for the .icns.
function squircle(size) {
  const r = size / 2;
  const points = [];
  for (let i = 0; i < 720; i++) {
    const t = (2 * Math.PI * i) / 720;
    const [c, s] = [Math.cos(t), Math.sin(t)];
    const x = Math.sign(c) * Math.abs(c) ** (2 / 5) * r + r;
    const y = Math.sign(s) * Math.abs(s) ** (2 / 5) * r + r;
    points.push(`${x.toFixed(2)},${y.toFixed(2)}`);
  }
  return Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}"><polygon points="${points.join(" ")}"/></svg>`,
  );
}

async function rounded(size, out) {
  await sharp(SRC, { density: 300 })
    .resize(size, size)
    .composite([{ input: squircle(size), blend: "dest-in" }])
    .png()
    .toFile(out);
  console.log(out);
}

await rounded(64, "public/favicon.png");
await rounded(256, "public/assets/icon-256.png");
await rounded(512, "public/assets/icon-512.png");
await sharp(SRC, { density: 300 }).resize(180, 180).flatten({ background: "#16171b" }).png().toFile("public/assets/apple-touch-icon.png");
console.log("public/assets/apple-touch-icon.png");
