import type { Configuration } from "electron-builder"

const channel = (() => {
  const raw = process.env.PIXICODE_CHANNEL
  if (raw === "dev" || raw === "beta" || raw === "prod") return raw
  return "dev"
})()

const getBase = (): Configuration => ({
  artifactName: "pixicode-electron-${os}-${arch}.${ext}",
  directories: {
    output: "dist",
    buildResources: "resources",
  },
  files: ["out/**/*", "resources/**/*"],
  extraResources: [
    {
      from: "resources/",
      to: "",
      filter: ["pixicode-cli*"],
    },
    {
      from: "native/",
      to: "native/",
      filter: ["index.js", "index.d.ts", "build/Release/mac_window.node", "swift-build/**"],
    },
  ],
  mac: {
    category: "public.app-category.developer-tools",
    icon: `resources/icons/icon.icns`,
    hardenedRuntime: true,
    gatekeeperAssess: false,
    entitlements: "resources/entitlements.plist",
    entitlementsInherit: "resources/entitlements.plist",
    notarize: true,
    target: ["dmg", "zip"],
  },
  dmg: {
    sign: true,
  },
  protocols: {
    name: "PixiCode",
    schemes: ["pixicode"],
  },
  win: {
    icon: `resources/icons/icon.ico`,
    target: ["nsis"],
  },
  nsis: {
    oneClick: false,
    allowToChangeInstallationDirectory: true,
    installerIcon: `resources/icons/icon.ico`,
    installerHeaderIcon: `resources/icons/icon.ico`,
  },
  linux: {
    icon: `resources/icons`,
    category: "Development",
    target: ["AppImage", "deb", "rpm"],
  },
})

function getConfig() {
  const base = getBase()

  switch (channel) {
    case "dev": {
      return {
        ...base,
        appId: "ai.pixicode.desktop.dev",
        productName: "PixiCode Dev",
        rpm: { packageName: "pixicode-dev" },
      }
    }
    case "beta": {
      return {
        ...base,
        appId: "ai.pixicode.desktop.beta",
        productName: "PixiCode Beta",
        protocols: { name: "PixiCode Beta", schemes: ["pixicode"] },
        publish: { provider: "github", owner: "anomalyco", repo: "pixicode-beta", channel: "latest" },
        rpm: { packageName: "pixicode-beta" },
      }
    }
    case "prod": {
      return {
        ...base,
        appId: "ai.pixicode.desktop",
        productName: "PixiCode",
        protocols: { name: "PixiCode", schemes: ["pixicode"] },
        publish: { provider: "github", owner: "anomalyco", repo: "pixicode", channel: "latest" },
        rpm: { packageName: "pixicode" },
      }
    }
  }
}

export default getConfig()
