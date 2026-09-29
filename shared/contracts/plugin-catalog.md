# Plugin catalog contract v2

The plugin catalog contract describes the built-in extension catalog that a
host ships with Core. Schema v2 introduces per-category contribution
structures (themes, icon themes, fonts, debug adapters) and derives browsing
categories from them, following Zed's extension model: the manifest never
declares categories; categories are computed from non-empty contributions so
a listing cannot drift from the contributions that back it.

## Entrypoint kinds

| Kind | Meaning |
| --- | --- |
| `builtIn` | Host core capability compiled into the host. **Not uninstallable.** |
| `bundled` | Shipped with the host, but **uninstallable by the user** (e.g. a bundled theme group). Same shape as `builtIn`: only `targetName` is accepted. |
| `nativeBundle` | Third-party native package, referenced by bundle path. Not distributable yet. |

Uninstall is a host-side lifecycle, not a catalog fact: Core serves the same
catalog either way and the host records what the user removed. The host marks
`bundled` entries uninstallable and keeps the ordered
`uninstalledExtensions` list in its settings document.

## Catalog document

The catalog is a JSON document with:

| Field | Type | Meaning |
| --- | --- | --- |
| `schemaVersion` | integer | Must be `2` for this Core build. |
| `hostVersion` | string | Exact host version the catalog was assembled for (strict `major.minor.patch`). |
| `pluginAPIVersion` | integer | Plugin API level shared by every package (`1`). |
| `plugins` | array | Package manifests sorted by stable `id`. |

## Package manifest (v2)

One entry per plugin package:

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | Stable catalog key, sorted across the catalog. |
| `displayName` | string | Name shown by host applications. |
| `description` | string? | Optional short listing summary. |
| `version` | string | Strict `major.minor.patch`. |
| `apiVersion` | integer | Must equal `pluginAPIVersion`. |
| `hostCompatibility` | object | Inclusive `minimum`, optional exclusive `maximumExclusive`. |
| `vendor` | object | `id`, `displayName`, `signatureRequirement` (`sameTeamAsHost` only). |
| `entrypoint` | object | `builtIn` or `bundled` (both with `targetName`), or `nativeBundle` (with `bundleIdentifier`, `principalClass`, `bundlePath`). |
| `moduleIDs` | array | Stable module identifiers owned by this package, sorted, globally unique. |
| `themes` | array | Optional. `{ name, path }` sorted by `(name, path)`. |
| `iconThemes` | array | Optional. `{ name, path }` sorted by `(name, path)`. |
| `fonts` | array | Optional. `{ family, path }` sorted by `(family, path)`; one family may list several weight files. |
| `debugAdapters` | array | Optional. `{ id, name }` sorted by `(id, name)`. |
| `languageSupports` | array | Optional. File recognition plus `languageServerModuleID` / `executionModuleID` / `testingModuleID` / `debugModuleID` slots that must reference owned modules. |

Contribution `path` values are bundle-relative: no leading `/`, no backslash,
no empty or `..` components. For `builtIn` packages the paths are
informational references to the host's bundled resources.

## Categories (derived)

A plugin's categories are computed from its non-empty contributions:

| Contribution | Category |
| --- | --- |
| `debugAdapters` | `debug` |
| `fonts` | `fonts` |
| `iconThemes` | `icon-themes` |
| `languageSupports` | `languages` |
| any `languageSupports[].languageServerModuleID` | `lsp` |
| `themes` | `themes` |

The canonical category order is `debug, fonts, icon-themes, languages, lsp,
themes`.

## Command surface

`plugins.listCatalog` returns the validated built-in catalog:

- Request payload: `{ "hostVersion": "<strict semver>" }` (the host passes its
  own version; it must equal the catalog's `hostVersion`).
- Response `data`: the catalog document, with each entry in `plugins` carrying
  an extra derived `categories` array.
- Failure codes: `invalid_request` (payload or `hostVersion` malformed),
  `parse_failed` (catalog JSON invalid, shape unsupported, or any validation
  rule failed).

The catalog itself is bundled with Core
(`shared/fixtures/plugins/official-v2.json`, embedded at compile time); there
is no remote registry in v2 and no install/uninstall surface yet.

## Fixtures

- `plugins/official-v2.json` — the official built-in catalog (validated by
  `scripts/verify-shared-contracts.sh` and Core tests).
- `plugins/language-support-v2.json` — language support + shared module
  ownership scenario.
