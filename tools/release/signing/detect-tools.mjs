/**
 * Detect Authenticode-capable signing tools without assuming install (W2.4.2-A Part 7).
 *
 * Candidates:
 *  - signtool.exe (Windows SDK)
 *  - AzureSignTool (dotnet tool / PATH)
 *  - PowerShell Set-AuthenticodeSignature (built-in on Windows)
 */

import { spawnSync, execSync } from "node:child_process";
import { existsSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import os from "node:os";

function trySpawn(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, {
    encoding: "utf8",
    windowsHide: true,
    timeout: opts.timeout ?? 15_000,
    shell: opts.shell === true,
    env: process.env,
  });
  return {
    ok: r.status === 0 && !r.error,
    status: r.status,
    stdout: (r.stdout || "").trim(),
    stderr: (r.stderr || "").trim(),
    error: r.error ? String(r.error.message || r.error) : null,
  };
}

function walkForSigntool(root, maxDepth = 4) {
  const found = [];
  if (!root || !existsSync(root)) return found;
  const stack = [{ dir: root, depth: 0 }];
  while (stack.length && found.length < 5) {
    const { dir, depth } = stack.pop();
    let entries;
    try {
      entries = readdirSync(dir);
    } catch {
      continue;
    }
    for (const name of entries) {
      const full = path.join(dir, name);
      let st;
      try {
        st = statSync(full);
      } catch {
        continue;
      }
      if (st.isFile() && /^signtool\.exe$/i.test(name)) {
        found.push(full);
      } else if (st.isDirectory() && depth < maxDepth) {
        // Prefer bin\x64\signtool.exe style trees
        if (
          /^(bin|x64|x86|10\.|App Certification Kit)/i.test(name) ||
          depth < 2
        ) {
          stack.push({ dir: full, depth: depth + 1 });
        }
      }
    }
  }
  return found;
}

export function findSignToolCandidates() {
  const candidates = [];
  const which = trySpawn(
    process.platform === "win32" ? "where.exe" : "which",
    ["signtool"],
  );
  if (which.ok && which.stdout) {
    for (const line of which.stdout.split(/\r?\n/)) {
      if (line.trim()) candidates.push(line.trim());
    }
  }
  if (process.platform === "win32") {
    const roots = [
      path.join(process.env["ProgramFiles(x86)"] || "", "Windows Kits", "10", "bin"),
      path.join(process.env.ProgramFiles || "", "Windows Kits", "10", "bin"),
      path.join(
        process.env["ProgramFiles(x86)"] || "",
        "Microsoft SDKs",
        "ClickOnce",
        "SignTool",
      ),
    ];
    for (const r of roots) {
      candidates.push(...walkForSigntool(r, 5));
    }
  }
  const uniq = [...new Set(candidates)];
  // prefer x64 over x86/arm64 when multiple SDK copies exist
  const score = (p) => {
    const n = String(p).toLowerCase();
    if (n.includes("\\x64\\") || n.includes("/x64/")) return 0;
    if (n.includes("\\x86\\") || n.includes("/x86/")) return 1;
    if (n.includes("\\arm64\\") || n.includes("/arm64/")) return 2;
    return 3;
  };
  return uniq.sort((a, b) => score(a) - score(b));
}

export function probeSignTool(exePath) {
  if (!exePath || !existsSync(exePath)) {
    return { available: false, path: null, version: null };
  }
  const r = trySpawn(exePath, ["/?"], { timeout: 10_000 });
  const text = `${r.stdout}\n${r.stderr}`;
  const ver =
    text.match(/SignTool\s+Version[:\s]+([0-9.]+)/i)?.[1] ||
    text.match(/Version[:\s]+([0-9.]+)/i)?.[1] ||
    null;
  // signtool /? often exits non-zero; presence of usage text is enough
  const available =
    /Sign Tool|signtool|Usage:/i.test(text) || r.ok || existsSync(exePath);
  return { available, path: exePath, version: ver, rawHelpOk: r.ok };
}

export function probeAzureSignTool() {
  const r = trySpawn("AzureSignTool", ["--version"], { shell: true });
  if (r.ok) {
    return {
      available: true,
      path: "AzureSignTool",
      version: r.stdout.split(/\r?\n/)[0] || r.stdout || null,
    };
  }
  const dotnet = trySpawn(
    "dotnet",
    ["tool", "run", "AzureSignTool", "--version"],
    { shell: true },
  );
  if (dotnet.ok) {
    return {
      available: true,
      path: "dotnet tool run AzureSignTool",
      version: dotnet.stdout.split(/\r?\n/)[0] || null,
    };
  }
  return { available: false, path: null, version: null };
}

export function probePowerShellAuthenticode() {
  if (process.platform !== "win32") {
    return { available: false, path: null, version: null };
  }
  const r = trySpawn(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "$PSVersionTable.PSVersion.ToString(); Get-Command Set-AuthenticodeSignature | Select-Object -ExpandProperty Name",
    ],
    { timeout: 20_000 },
  );
  if (!r.ok) {
    return { available: false, path: null, version: null, error: r.stderr || r.error };
  }
  const lines = r.stdout.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  return {
    available: lines.some((l) => /Set-AuthenticodeSignature/i.test(l)),
    path: "powershell:Set-AuthenticodeSignature",
    version: lines[0] || null,
  };
}

/**
 * Full environment tool inventory.
 */
export function detectSigningTools() {
  const platform = `${process.platform}/${process.arch}`;
  const osRelease = os.release();
  const signToolPaths = findSignToolCandidates();
  const signTools = signToolPaths.map(probeSignTool).filter((t) => t.available);
  const azure = probeAzureSignTool();
  const powershell = probePowerShellAuthenticode();

  const preferred =
    signTools[0] ||
    (azure.available ? azure : null) ||
    (powershell.available ? powershell : null);

  return {
    platform,
    osRelease,
    windows: process.platform === "win32",
    tools: {
      signtool: signTools[0] || { available: false, path: null, version: null },
      signtoolCandidates: signTools,
      azureSignTool: azure,
      powershellAuthenticode: powershell,
    },
    preferred: preferred
      ? {
          kind: signTools[0]
            ? "signtool"
            : azure.available
              ? "AzureSignTool"
              : "powershell",
          path: preferred.path,
          version: preferred.version,
        }
      : null,
    anyAvailable: Boolean(preferred),
  };
}
