# Current-source verification anchors (2026-09-21)

These sources support **design constraints**, not an assertion that a package/driver/model is already installed or that
a fork has been built. Recheck every version and artifact at implementation time.

- WinGet install and exact package IDs: https://learn.microsoft.com/en-us/windows/package-manager/winget/install
- WinGet search and source: https://learn.microsoft.com/en-us/windows/package-manager/winget/search
- WASAPI render endpoint loopback: https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording
- Windows application/process loopback sample:
  https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/
- OIDC overview: https://openid.net/developers/how-connect-works/
- Candy Icons upstream and GPL-3.0 notice: https://github.com/EliverLara/candy-icons
- NVIDIA NVML device queries: https://docs.nvidia.com/deploy/nvml-api/api/group__nvmlDeviceQueries.html
- NVIDIA TensorRT-LLM upstream and license: https://github.com/NVIDIA/TensorRT-LLM
- NVIDIA Triton Inference Server upstream and license: https://github.com/triton-inference-server/server
- NVIDIA Riva support matrix:
  https://docs.nvidia.com/deeplearning/riva/user-guide/docs/support-matrix/support-matrix.html

**Packaging gate:** source availability, cost, supported OS/GPU, model license and redistribution rights must be
verified independently for every NVIDIA component and every catalog item. Some NVIDIA components are open-source
projects, some contain separately licensed dependencies, some may be trial or platform limited. Make open and free
options the primary integrated path wherever viable. Do not state that all NVIDIA AI software is open-source.

**Source maturity:** original 12_RESEARCH_NOTES_AND_SOURCES.md is historical planning dated 2026-09-19. The implementing
agent must revalidate upstream releases, exact package IDs and product availability on the target system.
