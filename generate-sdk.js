const codegen = require("@cosmwasm/ts-codegen").default; // Add `.default`

async function generateSDK() {
  try {
    await codegen({
      contracts: [
        {
          name: "truth-markets-contract-fixed",
          dir: "./schema",
        },
      ],
      outPath: "./src/types/",

      options: {
        bundle: {
          bundleFile: "index.ts",
          scope: "contracts",
        },
        types: {
          enabled: true,
        },
        client: {
          enabled: true,
        },
        reactQuery: {
          enabled: true,
          optionalClient: true,
          version: "v4",
          mutations: true,
          queryKeys: true,
          queryFactory: true,
        },
        recoil: {
          enabled: false,
        },
        messageComposer: {
          enabled: false,
        },
        messageBuilder: {
          enabled: false,
        },
        useContractsHook: {
          enabled: true,
        },
      },
    });

    console.log("✨ TypeScript SDK generation complete!");
  } catch (error) {
    console.error("❌ SDK generation failed:", error);
    process.exit(1);
  }
}

generateSDK();
