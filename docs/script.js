const repository = "3xian/PinkDown";
const releasesUrl = `https://github.com/${repository}/releases/latest`;

function preferredAsset(assets) {
  const platform = navigator.platform.toLowerCase();
  const userAgent = navigator.userAgent.toLowerCase();
  const isWindows = platform.includes("win") || userAgent.includes("windows");

  if (isWindows) {
    return assets.find((asset) => asset.name === "pinkdown-windows-x64-setup.exe");
  }

  return null;
}

async function configureDownload() {
  const primary = document.querySelector("#primary-download");
  const footer = document.querySelector("#footer-download");
  const label = document.querySelector("#download-label");
  const note = document.querySelector("#release-note");

  try {
    const response = await fetch(`https://api.github.com/repos/${repository}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });

    if (!response.ok) throw new Error("Release lookup failed");

    const release = await response.json();
    const asset = preferredAsset(release.assets || []);
    const version = release.tag_name || "latest";
    const isMac = navigator.platform.toLowerCase().includes("mac") || navigator.userAgent.toLowerCase().includes("macintosh");

    note.textContent = `${version} · Free and open source · Windows & macOS`;

    if (asset) {
      const isMac = asset.name.endsWith(".dmg");
      primary.href = asset.browser_download_url;
      footer.href = asset.browser_download_url;
      label.textContent = `Download for ${isMac ? "macOS" : "Windows"}`;
      footer.textContent = `Download ${version}`;
    } else if (isMac) {
      label.textContent = "Download for macOS";
      footer.textContent = `Choose ${version} for your Mac`;
    }
  } catch {
    primary.href = releasesUrl;
    footer.href = releasesUrl;
  }
}

document.querySelectorAll("[data-external]").forEach((link) => {
  link.setAttribute("target", "_blank");
  link.setAttribute("rel", "noreferrer");
});

configureDownload();
