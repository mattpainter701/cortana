import { tool } from "@opencode-ai/plugin"
import type { Plugin } from "@opencode-ai/plugin"

const DEFAULT_CORTANA_BIN = "cortana"

type SummaryMode = "text" | "code" | "diff"

function cortanaBin() {
  return process.env.CORTANA_BIN || DEFAULT_CORTANA_BIN
}

export const CortanaPlugin: Plugin = async ({ $, client }) => {
  await client.app.log({
    body: {
      service: "cortana-plugin",
      level: "info",
      message: "Cortana OpenCode addon loaded",
      extra: { bin: cortanaBin() },
    },
  })

  return {
    "shell.env": async (_input, output) => {
      output.env.CORTANA_BIN = cortanaBin()
    },

    tool: {
      cortana_presence: tool({
        description:
          "Show Cortana's terminal-native hologram startup banner. Use this when the user wants the visual Cortana presence from inside OpenCode.",
        args: {},
        async execute() {
          const result = await $`${cortanaBin()} session start`.text()
          return result
        },
      }),

      cortana_recap: tool({
        description:
          "Return Cortana's current local session recap as JSON without spending LLM tokens.",
        args: {},
        async execute() {
          const result = await $`${cortanaBin()} recap --session current --format json`.text()
          return result
        },
      }),

      cortana_summarize: tool({
        description:
          "Summarize text/code/diff content locally with Cortana's zero-token summarizer.",
        args: {
          mode: tool.schema.enum(["text", "code", "diff"]),
          content: tool.schema.string(),
        },
        async execute(args: { mode: SummaryMode; content: string }) {
          const result = await $`${cortanaBin()} summarize --fast --mode ${args.mode} --json`.stdin(args.content).text()
          return result
        },
      }),
    },
  }
}
