---
plan name: build-cache
plan description: Faster image turnaround
plan status: done
---

## Idea
Reduce end-to-end Prism build time across local Docker builds, GitHub Actions, and Portainer by shrinking build context, maximizing layer reuse, and avoiding unnecessary multi-arch or full-image rebuilds on every push. The plan should keep the current pull-based deployment model for Portainer, but make CI the single place that produces versioned images with reusable cache layers and predictable triggers.

## Implementation
- Measure the current cold and warm timings for the website Docker builds, GitHub Actions publish workflow, and local compose startup so the plan can be validated against real numbers.
- Add root and website-level .dockerignore files to exclude node_modules, .next, dist, target, logs, git metadata, and other non-build inputs from the Docker build context.
- Refine the Dockerfiles so dependency manifests are copied first, dependency installation is isolated from source code changes, and BuildKit cache mounts are used for Cargo and npm layers.
- Rework the GitHub Actions workflow to use buildx cache-to/cache-from with GitHub Actions cache, separate publish from verification, and keep cache scopes isolated per image.
- Split faster validation paths from slower publish paths by using path filters, single-arch PR builds, and multi-arch main-branch publishes only when needed.
- Add a scheduled cache-warm workflow and cache cleanup strategy so main branch caches stay fresh without letting old branches consume cache quota.
- Verify the gains by comparing build durations before and after, then document the recommended local and CI commands for future maintenance.

## Required Specs
<!-- SPECS_START -->
- agents-guidelines
<!-- SPECS_END -->