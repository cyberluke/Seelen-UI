import "../../../../libs/core/styles/colors.css";
import "../../../../libs/core/styles/spacings.css";
import "../../../../libs/core/styles/shadows.css";

import { _invoke } from "./_tauri";
import "./ConsoleWrapper.ts";
import "./LivenessProve.ts";
import "./MainSetup.ts";
import "./PreventDefaults.ts";
import "./UxImprovements.ts";

_invoke("record_boot_stage", { stage: "bootstrap.module.loaded" }).catch(() => {});
