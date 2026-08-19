import {
  Rocket,
  Clapperboard,
  Keyboard,
  Share2,
  Sparkles,
  LifeBuoy,
  type LucideIcon,
} from "lucide-react";

export type GuideBlock =
  | { kind: "p"; text: string }
  | { kind: "ul"; items: string[] }
  | { kind: "ol"; items: string[] }
  | { kind: "code"; label?: string; text: string }
  | { kind: "note"; text: string };

export interface GuideSection {
  id: string;
  heading: string;
  blocks: GuideBlock[];
}

export interface Guide {
  slug: string;
  title: string;
  description: string;
  icon: LucideIcon;
  sections: GuideSection[];
}

export const guides: Guide[] = [
  {
    slug: "install-first-clip",
    title: "Install & record your first clip",
    description:
      "Download Prism, launch it, and save your first clip in under a minute.",
    icon: Rocket,
    sections: [
      {
        id: "install",
        heading: "Installing Prism",
        blocks: [
          {
            kind: "p",
            text: "Download the Prism installer for Windows from the Download page. Run the installer — no extra dependencies are required. On macOS, drag the app into your Applications folder.",
          },
          {
            kind: "p",
            text: "When you first launch Prism it will request screen-recording permission. macOS users must grant it in System Settings > Privacy & Security > Screen Recording, then restart Prism.",
          },
          {
            kind: "note",
            text: "Prism records your screen continuously into a memory buffer so you can save clips after they happen. Nothing is written to disk until you trigger a clip save.",
          },
        ],
      },
      {
        id: "first-clip",
        heading: "Saving your first clip",
        blocks: [
          {
            kind: "ol",
            items: [
              "Launch Prism — recording starts automatically if “Always-on recording” is enabled (default).",
              "Press the save-clip hotkey (Ctrl+Shift+X on Windows, Cmd+Shift+X on macOS).",
              "Wait a moment while the last few seconds are written to MP4.",
              "Open the Library to watch your clip.",
            ],
          },
          {
            kind: "p",
            text: "A notification appears when the clip is saved. The clip includes a JPEG thumbnail generated automatically, so it's easy to spot in the library.",
          },
        ],
      },
      {
        id: "customize",
        heading: "Tuning capture settings",
        blocks: [
          {
            kind: "p",
            text: "Open Settings > Recording to adjust how much gameplay is kept in the buffer (10 seconds up to 30 minutes), the resolution, FPS, and bitrate. Quality presets (Fast / Balanced / High) set all three at once — tweak any single value and it switches to a custom setup.",
          },
          {
            kind: "p",
            text: "The output directory defaults to your Videos folder. Change it in Settings > Recording > Output directory.",
          },
        ],
      },
    ],
  },
  {
    slug: "recording-clips",
    title: "Recording & managing clips",
    description:
      "How the shadow buffer works, how to find and organize your clips, and how the library is laid out.",
    icon: Clapperboard,
    sections: [
      {
        id: "how-it-works",
        heading: "How the shadow buffer works",
        blocks: [
          {
            kind: "p",
            text: "Prism continuously records your screen into a compressed H.264 buffer in memory — the shadow buffer. When you trigger a clip save, the last N seconds are exported to an MP4 on disk. The buffer holds roughly 7 minutes of 1080p video within a 256 MB memory budget before the oldest footage is overwritten.",
          },
          {
            kind: "p",
            text: "This means you never have to press record before a moment happens. You save clips after they happen — the footage is always there.",
          },
        ],
      },
      {
        id: "library",
        heading: "Using the clip library",
        blocks: [
          {
            kind: "ul",
            items: [
              "Search clips by name, description, or game.",
              "Sort by newest, oldest, name, size, or duration.",
              "Filter by upload status (uploaded / uploading / failed) or by game.",
              "Hover a clip card for quick actions: play, upload, copy share link, or delete.",
            ],
          },
          {
            kind: "p",
            text: "Click any clip to open its detail page, where you can play it in-app and edit its name, game tag, and description. These details are stored separately from the video file so they survive re-encoding and future editing features.",
          },
        ],
      },
      {
        id: "duration",
        heading: "Adjusting clip duration",
        blocks: [
          {
            kind: "p",
            text: "In Settings > Recording, drag the Clip length slider. Longer durations give more context per clip but use more of the shadow buffer (and larger clips upload more slowly). Changes apply to future clips only.",
          },
        ],
      },
      {
        id: "storage",
        heading: "Storage management",
        blocks: [
          {
            kind: "p",
            text: "In Settings > Storage you can cap how much disk space the library may use and enable auto-pruning so clips older than a chosen number of days are deleted automatically. An empty prune value disables it.",
          },
        ],
      },
    ],
  },
  {
    slug: "hotkeys",
    title: "Hotkeys & shortcuts",
    description:
      "The default keybindings, how to rebind them, and how to trigger actions from anywhere.",
    icon: Keyboard,
    sections: [
      {
        id: "defaults",
        heading: "Default hotkeys",
        blocks: [
          {
            kind: "p",
            text: "Global hotkeys work even when Prism is minimized to the tray or another app is in the foreground:",
          },
          {
            kind: "ul",
            items: [
              "Save clip — Ctrl+Shift+X (Cmd+Shift+X on macOS)",
              "Toggle recording — Ctrl+Shift+R (Cmd+Shift+R)",
              "Open library — Ctrl+Shift+L (Cmd+Shift+L)",
            ],
          },
        ],
      },
      {
        id: "rebind",
        heading: "Rebinding hotkeys",
        blocks: [
          {
            kind: "ol",
            items: [
              "Open Settings > Hotkeys.",
              "Click a hotkey row — it starts listening for a key chord.",
              "Press the new combination. It registers immediately.",
            ],
          },
          {
            kind: "p",
            text: "Bindings take effect instantly, no restart required. Use “Reset to defaults” to restore the original chords. Leave a binding empty to disable that action.",
          },
        ],
      },
      {
        id: "tray",
        heading: "System tray & menu",
        blocks: [
          {
            kind: "p",
            text: "Prism lives in your system tray while running. Right-click the tray icon for Save Clip, Open Library, Settings, and Quit. Left-clicking the icon brings the window to the foreground. If “Minimize to tray” is enabled (default), closing the window hides Prism to the tray instead of quitting.",
          },
        ],
      },
    ],
  },
  {
    slug: "sharing",
    title: "Sharing clips",
    description:
      "Upload clips to the cloud, copy share links, and control who can see them.",
    icon: Share2,
    sections: [
      {
        id: "connect",
        heading: "Connecting your account",
        blocks: [
          {
            kind: "p",
            text: "Sign in from the sidebar (or Settings > Cloud) using Google OAuth. The sign-in opens in your browser and hands back to Prism automatically via a prism:// deep link. If the automatic handoff doesn't work, paste the auth code manually from Settings > Cloud.",
          },
        ],
      },
      {
        id: "upload",
        heading: "Uploading a clip",
        blocks: [
          {
            kind: "ol",
            items: [
              "Open the Library and hover the clip you want to share.",
              "Click the upload button (cloud icon).",
              "Watch the progress bar on the card. Once uploaded, the card shows an “Uploaded” badge.",
              "Click the link icon to copy the share URL to your clipboard.",
            ],
          },
          {
            kind: "p",
            text: "Enable “Auto-upload” in Settings > Cloud to upload every saved clip automatically. You can also set how many uploads run in parallel (1–3). Failed uploads are queued and retried automatically.",
          },
        ],
      },
      {
        id: "visibility",
        heading: "Controlling who sees a clip",
        blocks: [
          {
            kind: "p",
            text: "Clips are private by default — only you can see them. From the web dashboard, open a clip and use Share to set its visibility:",
          },
          {
            kind: "ul",
            items: [
              "Public — visible on your public profile (goprism.studio/u/username) and via the share link.",
              "Unlisted — visible only to people with the link, not on your profile.",
              "Private — only you can view it.",
            ],
          },
        ],
      },
      {
        id: "profiles",
        heading: "Public profiles",
        blocks: [
          {
            kind: "p",
            text: "Your profile shows every public clip in a grid. Share links render as rich preview cards on Discord, X/Twitter, and other apps thanks to Open Graph meta tags.",
          },
        ],
      },
    ],
  },
  {
    slug: "auto-clip",
    title: "Auto-clipping with game detection",
    description:
      "Automatically capture highlights from CS2 and Rust — no hotkey required.",
    icon: Sparkles,
    sections: [
      {
        id: "overview",
        heading: "How auto-clipping works",
        blocks: [
          {
            kind: "p",
            text: "Prism detects which game is in the foreground and listens for in-game moments using read-only, anti-cheat-safe methods. When a moment happens it saves a clip automatically, exactly as if you'd pressed the hotkey. Enable it in Settings > General (Game detection) and Settings > Auto-clip (Auto-clipping).",
          },
          {
            kind: "note",
            text: "Prism never injects into a game or reads game memory. CS2 uses Valve's official Game State Integration API; Rust uses audio analysis of the game's sound output.",
          },
        ],
      },
      {
        id: "cs2",
        heading: "Counter-Strike 2",
        blocks: [
          {
            kind: "p",
            text: "CS2 integration uses Valve's official Game State Integration (GSI). Prism installs a small config file (gamestate_integration_prism.cfg) into your CS2 config folder automatically. Start CS2 once after installing Prism so the file is picked up.",
          },
          {
            kind: "ul",
            items: [
              "Events: kills, deaths, headshots, and round wins.",
              "Choose which events trigger clips and how long each clip should be.",
              "Change the GSI port in Settings > General if 4000 is already in use (restart Prism after).",
            ],
          },
        ],
      },
      {
        id: "rust",
        heading: "Rust",
        blocks: [
          {
            kind: "p",
            text: "On Windows, Prism captures Rust's game audio through WASAPI and analyzes it with FFT to detect gunfights, headshot dings, rockets, and C4. Adjust the audio sensitivity in Settings > Auto-clip if you get too many (or too few) clips.",
          },
          {
            kind: "ul",
            items: [
              "Events: gunfights, headshot dings, rockets/C4 explosions.",
              "Per-game clip durations for each event type (5–120 seconds).",
              "A global cooldown (5–120 seconds) prevents clip spam.",
            ],
          },
        ],
      },
      {
        id: "clips",
        heading: "Reviewing auto-clips",
        blocks: [
          {
            kind: "p",
            text: "Auto-clips land in your library just like manual clips, tagged with the detected game. A live “Detected / Waiting” badge on the Auto-clip settings page confirms whether Prism currently sees the game.",
          },
        ],
      },
    ],
  },
  {
    slug: "troubleshooting",
    title: "Troubleshooting & FAQ",
    description:
      "Fix common issues with permissions, recordings, uploads, and the cloud.",
    icon: LifeBuoy,
    sections: [
      {
        id: "permissions",
        heading: "Clips aren't saving / preview is black",
        blocks: [
          {
            kind: "ul",
            items: [
              "Windows: make sure your GPU drivers are up to date and that Prism has permission to capture the screen.",
              "macOS: grant Screen Recording permission in System Settings > Privacy & Security > Screen Recording, then restart Prism.",
              "Verify the output directory in Settings > Recording exists and is writable.",
            ],
          },
        ],
      },
      {
        id: "cpu",
        heading: "High CPU or stuttering",
        blocks: [
          {
            kind: "p",
            text: "Prism is designed to run at low CPU during background recording. If you notice stutter, try lowering the FPS or resolution in Settings > Recording, or disable system audio capture if you don't need it.",
          },
        ],
      },
      {
        id: "uploads",
        heading: "Uploads fail or get stuck",
        blocks: [
          {
            kind: "ul",
            items: [
              "Check that the server URL in Settings > Cloud is correct and reachable.",
              "Confirm you're signed in — uploads require a valid account.",
              "Failed uploads appear with a red “Failed” badge; retry them from the clip card.",
              "Large clips on slow connections may take a while — concurrent uploads (up to 3) are configurable.",
            ],
          },
        ],
      },
      {
        id: "faq",
        heading: "Frequently asked questions",
        blocks: [
          {
            kind: "p",
            text: "Where are my clips stored? In the output directory configured in Settings > Recording (default: your Videos folder), as .mp4 files plus a .jpg thumbnail.",
          },
          {
            kind: "p",
            text: "Is Prism safe to use with anti-cheat? Yes. Prism reads the screen and window titles only — it never injects code into a game or reads game memory.",
          },
          {
            kind: "p",
            text: "Can I use Prism without the cloud? Yes. Recording and clip saving work fully offline. The cloud is only needed for uploads and sharing.",
          },
        ],
      },
    ],
  },
];

export function getGuide(slug: string): Guide | undefined {
  return guides.find((guide) => guide.slug === slug);
}