import { Plugin, TFile, Notice } from "obsidian";
import { Client, GatewayIntentBits, Message } from "discord.js";
import { to_md } from "../pkg/parse_message";

/* ────────────────────  type & defaults  ──────────────────── */

interface Settings {
  /** Discord Bot token                         */ token: string;
  /** Channel ID whose messages will be saved   */ channel: string;
  /** Vault-relative folder for output md files */ dir: string;
}

const DEFAULT: Settings = {
  token: "",
  channel: "",
  dir: "DiscordLogs",
};

/* ────────────────────  main plugin  ──────────────────── */

export default class DiscordSync extends Plugin {
  private cfg!: Settings;
  private client!: Client<boolean>;

  /* ── Obsidian lifecycle: load ─────────────────────────── */
  async onload() {
    await this.loadSettings();

    /* Discord gateway */
    this.client = new Client({
      intents: [
        GatewayIntentBits.Guilds,
        GatewayIntentBits.GuildMessages,
        GatewayIntentBits.MessageContent,
      ],
    });

    this.client.on("messageCreate", this.handleMsg.bind(this));

    try {
      await this.client.login(this.cfg.token);
      new Notice("Discord Sync: connected");
    } catch (e) {
      new Notice(`Discord Sync: login failed - ${(e as Error).message}`);
    }

    /* quick JSON-based settings editor */
    this.addCommand({
      id: "open-discord-sync-config",
      name: "Discord Sync: Edit JSON config",
      callback: async () => {
        const p = `${this.manifest.dir}/discord-sync-config.json`;
        const f = await this.app.vault.createOrOpen(
          p,
          JSON.stringify(this.cfg, null, 2)
        );
        this.app.workspace.getLeaf(true).openFile(f);
      },
    });
  }

  /* ── shutdown */
  onunload() {
    this.client?.destroy();
  }

  /* ────────────────────  message handler  ──────────────────── */
  private async handleMsg(m: Message) {
    if (m.author.bot || m.channelId !== this.cfg.channel) return;

    const md = to_md(
      m.author.username,
      m.cleanContent,
      m.createdAt.toISOString()
    );

    try {
      await this.appendToVault(md);
      await m.react("✅").catch(() => {});
    } catch (e) {
      await m.react("❌").catch(() => {});
      console.error("Discord Sync write error:", e);
    }
  }

  /* ────────────────────  vault helpers  ──────────────────── */
  private async appendToVault(line: string) {
    const date = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
    const path = `${this.cfg.dir}/${this.cfg.channel}/${date}.md`;

    if (this.app.vault.getAbstractFileByPath(path)) {
      const file = this.app.vault.getAbstractFileByPath(path) as TFile;
      await this.app.vault.append(file, line);
    } else {
      await this.app.vault.create(path, `# ${date}\n\n${line}`);
    }
  }

  /* ────────────────────  settings I/O  ──────────────────── */
  private async loadSettings() {
    this.cfg = Object.assign({}, DEFAULT, await this.loadData());
  }
  async saveSettings() {
    await this.saveData(this.cfg);
  }
}
