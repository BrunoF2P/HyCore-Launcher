import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

// Configuration
const REPO_URL = "https://github.com/BrunoF2P/hycore";
const TAURI_CONF_PATH = path.join(process.cwd(), 'src-tauri', 'tauri.conf.json');
const RELEASE_DIR = path.join(process.cwd(), 'src-tauri', 'target', 'release', 'bundle');

async function main() {
    console.log("🔍 Reading configuration...");

    if (!fs.existsSync(TAURI_CONF_PATH)) {
        console.error("❌ tauri.conf.json not found!");
        process.exit(1);
    }

    const tauriConf = JSON.parse(fs.readFileSync(TAURI_CONF_PATH, 'utf-8'));
    const version = tauriConf.version;

    console.log(`📦 Detected Version: ${version}`);

    const updateData = {
        version: version,
        notes: "Update generated via script.",
        pub_date: new Date().toISOString(),
        platforms: {}
    };

    // Helper to find signature
    const findSignature = (platformDir, ext) => {
        const dir = path.join(RELEASE_DIR, platformDir);
        if (!fs.existsSync(dir)) return null;

        const files = fs.readdirSync(dir);
        const sigFile = files.find(f => f.endsWith(`${ext}.sig`));
        const binFile = files.find(f => f.endsWith(ext));

        if (binFile) {
            let sigContent = "SIGNATURE_NOT_FOUND_PLEASE_SIGN_MANUALLY";
            if (sigFile && fs.existsSync(path.join(dir, sigFile))) {
                sigContent = fs.readFileSync(path.join(dir, sigFile), 'utf-8');
            } else {
                console.warn(`⚠️  Warning: No signature found for ${binFile}. You will need to sign this manually for the updater to accept it.`);
            }

            return {
                signature: sigContent,
                url: `${REPO_URL}/releases/download/v${version}/${binFile}`
            };
        }
        return null;
    };

    console.log("🕵️  Looking for build artifacts...");

    // Linux AppImage
    const linuxData = findSignature('appimage', '.AppImage') || findSignature('deb', '.deb');
    if (linuxData) {
        updateData.platforms['linux-x86_64'] = linuxData;
        console.log("✅ Found Linux Artifact (AppImage or Deb)");
    }

    // Windows MSI/NSIS
    const winData = findSignature('nsis', '.exe') || findSignature('msi', '.msi');
    if (winData) {
        updateData.platforms['windows-x86_64'] = winData;
        console.log("✅ Found Windows Installer");
    }

    // macOS
    const macData = findSignature('macos', '.app.tar.gz'); // Tauri bundles as tar.gz for updates usually, checking basics
    if (macData) {
        updateData.platforms['darwin-x86_64'] = macData;
        updateData.platforms['darwin-aarch64'] = macData; // Assuming universal or specific logic needed
        console.log("✅ Found macOS Bundle");
    }

    if (Object.keys(updateData.platforms).length === 0) {
        console.warn("⚠️  No artifacts found! Did you run 'bun tauri build'?");
    }

    const outputPath = path.join(process.cwd(), 'latest.json');
    fs.writeFileSync(outputPath, JSON.stringify(updateData, null, 2));

    console.log(`\n🎉 Generated ${outputPath}`);
    console.log("---------------------------------------------------");
    console.log(JSON.stringify(updateData, null, 2));
    console.log("---------------------------------------------------");
    console.log("👉 Upload the binary files to GitHub Releases.");
    console.log("👉 Upload this 'latest.json' to the location defined in tauri.conf.json.");
}

main();
