#!/usr/bin/env node
/**
 * Version management script for Kizuna Node.js bindings
 * 
 * Ensures version consistency between:
 * - package.json
 * - Cargo.toml
 * - README.md
 */

const fs = require('fs');
const path = require('path');

const BINDINGS_DIR = path.join(__dirname, '..');
const ROOT_DIR = path.join(BINDINGS_DIR, '..', '..');

function readJSON(filePath) {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function writeJSON(filePath, data) {
    fs.writeFileSync(filePath, JSON.stringify(data, null, 2) + '\n');
}

function readTOML(filePath) {
    const content = fs.readFileSync(filePath, 'utf8');
    const versionMatch = content.match(/version\s*=\s*"([^"]+)"/);
    return versionMatch ? versionMatch[1] : null;
}

function updateTOML(filePath, newVersion) {
    let content = fs.readFileSync(filePath, 'utf8');
    content = content.replace(
        /version\s*=\s*"[^"]+"/,
        `version = "${newVersion}"`
    );
    fs.writeFileSync(filePath, content);
}

function updateREADME(filePath, newVersion) {
    let content = fs.readFileSync(filePath, 'utf8');
    // Update version badges or mentions
    content = content.replace(
        /version-\d+\.\d+\.\d+-/g,
        `version-${newVersion}-`
    );
    fs.writeFileSync(filePath, content);
}

function main() {
    const args = process.argv.slice(2);

    if (args.length === 0) {
        console.log('Usage: node version.js <new-version>');
        console.log('Example: node version.js 0.2.0');
        process.exit(1);
    }

    const newVersion = args[0];

    // Validate version format
    if (!/^\d+\.\d+\.\d+(-[a-z0-9.]+)?$/.test(newVersion)) {
        console.error('Error: Invalid version format. Use semantic versioning (e.g., 1.0.0)');
        process.exit(1);
    }

    console.log(`Updating version to ${newVersion}...\n`);

    // Update package.json
    const packagePath = path.join(BINDINGS_DIR, 'package.json');
    const packageData = readJSON(packagePath);
    const oldVersion = packageData.version;
    packageData.version = newVersion;
    writeJSON(packagePath, packageData);
    console.log(`✓ Updated package.json: ${oldVersion} → ${newVersion}`);

    // Update Cargo.toml
    const cargoPath = path.join(ROOT_DIR, 'Cargo.toml');
    if (fs.existsSync(cargoPath)) {
        const cargoVersion = readTOML(cargoPath);
        updateTOML(cargoPath, newVersion);
        console.log(`✓ Updated Cargo.toml: ${cargoVersion} → ${newVersion}`);
    }

    // Update README.md
    const readmePath = path.join(BINDINGS_DIR, 'README.md');
    if (fs.existsSync(readmePath)) {
        updateREADME(readmePath, newVersion);
        console.log(`✓ Updated README.md`);
    }

    console.log('\nVersion update complete!');
    console.log('\nNext steps:');
    console.log('1. Review the changes');
    console.log('2. Commit: git commit -am "chore: bump version to ' + newVersion + '"');
    console.log('3. Tag: git tag v' + newVersion);
    console.log('4. Push: git push && git push --tags');
}

if (require.main === module) {
    main();
}

module.exports = { readJSON, writeJSON, readTOML, updateTOML };
