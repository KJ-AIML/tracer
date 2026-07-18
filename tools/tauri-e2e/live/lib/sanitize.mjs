/**
 * Artifact / evidence sanitization for Live Grok GUI (W2.3-B).
 * Never persist credentials, tokens, or private path segments.
 */

/**
 * @param {string|null|undefined} text
 * @returns {string|null|undefined}
 */
export function sanitizeArtifactText(text) {
  if (text == null) return text;
  return String(text)
    .replace(/(Authorization:\s*)(Bearer\s+)?[^\s"']+/gi, "$1$2[REDACTED]")
    .replace(
      /(api[_-]?key|token|secret|password|credential)\s*[:=]\s*[^\s"']+/gi,
      "$1=[REDACTED]",
    )
    .replace(/([A-Za-z]:\\Users\\)[^\\"']+/g, "$1[USER]")
    .replace(/(\/Users\/)[^\/"']+/g, "$1[USER]")
    .replace(/sk-[A-Za-z0-9]{10,}/g, "[REDACTED]")
    .replace(/Bearer\s+[A-Za-z0-9._\-]{10,}/g, "Bearer [REDACTED]");
}

/**
 * @param {unknown} value
 * @returns {unknown}
 */
export function sanitizeJsonValue(value) {
  if (typeof value === "string") return sanitizeArtifactText(value);
  if (Array.isArray(value)) return value.map(sanitizeJsonValue);
  if (value && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) {
      if (/api[_-]?key|token|secret|password|credential|authorization/i.test(k)) {
        out[k] = "[REDACTED]";
      } else {
        out[k] = sanitizeJsonValue(v);
      }
    }
    return out;
  }
  return value;
}