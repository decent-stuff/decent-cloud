import fs from "fs/promises";
import { existsSync } from "fs";
import https from "https";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const images = {
  "hero.jpg":
    "https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=1920&q=80",
  "background.jpg":
    "https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=1920&q=80",
  "servers.jpg":
    "https://images.unsplash.com/photo-1558494949-ef010cbdcc31?w=800&q=80",
  "reputation.jpg":
    "https://images.unsplash.com/photo-1521790797524-b2497295b8a0?w=800&q=80",
  "security.jpg":
    "https://images.unsplash.com/photo-1563013544-824ae1b704d3?w=800&q=80",
};

async function downloadImage(url, filename) {
  const dir = path.join(process.cwd(), "public", "images");
  const filepath = path.join(dir, filename);

  // Check if file already exists
  if (existsSync(filepath)) {
    console.log(`Skipping ${filename} - already exists`);
    return;
  }

  await fs.mkdir(dir, { recursive: true });

  return new Promise((resolve, reject) => {
    console.log(`Downloading ${filename}...`);
    https
      .get(url, (response) => {
        if (response.statusCode !== 200) {
          reject(
            new Error(`Failed to download ${url}: ${response.statusCode}`)
          );
          return;
        }

        const fileStream = fs.createWriteStream(filepath);
        response.pipe(fileStream);

        fileStream.on("finish", () => {
          fileStream.close();
          console.log(`Downloaded ${filename}`);
          resolve();
        });
      })
      .on("error", reject);
  });
}

async function downloadAll() {
  try {
    console.log("Checking and downloading images...");
    await Promise.all(
      Object.entries(images).map(([filename, url]) =>
        downloadImage(url, filename)
      )
    );
    console.log("Image check/download complete!");
  } catch (error) {
    console.error("Error downloading images:", error);
    process.exit(1);
  }
}

downloadAll();
