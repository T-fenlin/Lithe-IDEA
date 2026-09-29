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
| `bundled` | Shipped with the host, but **uninstallable by the user** (bundled theme groups and font families). Same shape as `builtIn`: only `targetName` is accepted. |
| `nativeBundle` | Third-party native package, referenced by bundle path. Not distributable yet. |

Uninstall is a host-side lifecycle, not a catalog fact: Core serves the same
catalog either way and the host records what the user removed. The host marks
`bundled` entries uninstallable and keeps the ordered
`uninstalledExtensions` list in its settings document.

Removal takes effect differently per category, which the host surfaces in its
extensions page:

- **Themes** apply immediately — the theme registry watches the load directory
  and re-applies the current theme on reload.
- **Fonts** apply on the next start — GPUI's `TextSystem::add_fonts` only adds,
  so an already registered family cannot be removed while the app runs.
- **Icon sets** apply immediately — switching only swaps the lookup table. Unlike
  themes and fonts, an icon set has **no package files on disk**: it is compiled
  into the host (`assets/icons/<package>/`), so uninstalling one marks it
  unavailable (it disappears from the chooser and the active set falls back to
  the built-in `idea` package) rather than deleting files.

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
| `icons` | array | Optional. `{ name, path }` sorted by `(name, path)`. |
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
| `icons` | `icons` |
| `languageSupports` | `languages` |
| any `languageSupports[].languageServerModuleID` | `lsp` |
| `themes` | `themes` |

The canonical category order is `debug, fonts, icons, languages, lsp,
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

## Language contributions (current state)

`languageSupports` models **file recognition plus capability-module ownership**
(language server / execution / testing / debug). It does **not** model syntax
highlighting yet: grammars and `.scm` queries are a host-build concern today
(see the feature flags in `lithe-gpui/crates/editor/Cargo.toml` and the extra
registrations in `lithe-gpui/crates/editor/src/languages.rs`). A future schema
revision would add a grammar dimension (grammar source + query paths) so a
highlight-only language can be contributed the way Zed does it.

`fonts[].path` values follow the same rule as themes: `builtIn` families point
at the host's bundled asset root (`assets/fonts/...`), `bundled` families at
their package directory (`extensions/fonts/<package>/...`). Bundled font
packages ship their license text as a package file (OFL requires the license to
travel with the fonts).
