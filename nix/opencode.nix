{
  lib,
  stdenvNoCC,
  callPackage,
  bun,
  sysctl,
  makeBinaryWrapper,
  models-dev,
  ripgrep,
  installShellFiles,
  versionCheckHook,
  writableTmpDirAsHomeHook,
  node_modules ? callPackage ./node-modules.nix { },
}:
stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "pixicode";
  inherit (node_modules) version src;
  inherit node_modules;

  nativeBuildInputs = [
    bun
    installShellFiles
    makeBinaryWrapper
    models-dev
    writableTmpDirAsHomeHook
  ];

  configurePhase = ''
    runHook preConfigure

    cp -R ${finalAttrs.node_modules}/. .

    runHook postConfigure
  '';

  env.MODELS_DEV_API_JSON = "${models-dev}/dist/_api.json";
  env.PIXICODE_DISABLE_MODELS_FETCH = true;
  env.PIXICODE_VERSION = finalAttrs.version;
  env.PIXICODE_CHANNEL = "local";

  buildPhase = ''
    runHook preBuild

    cd ./packages/pixicode
    bun --bun ./script/build.ts --single --skip-install
    bun --bun ./script/schema.ts schema.json

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    install -Dm755 dist/pixicode-*/bin/pixicode $out/bin/pixicode
    install -Dm644 schema.json $out/share/pixicode/schema.json

    wrapProgram $out/bin/pixicode \
      --prefix PATH : ${
        lib.makeBinPath (
          [
            ripgrep
          ]
          # bun runs sysctl to detect if dunning on rosetta2
          ++ lib.optional stdenvNoCC.hostPlatform.isDarwin sysctl
        )
      }

    runHook postInstall
  '';

  postInstall = lib.optionalString (stdenvNoCC.buildPlatform.canExecute stdenvNoCC.hostPlatform) ''
    # trick yargs into also generating zsh completions
    installShellCompletion --cmd pixicode \
      --bash <($out/bin/pixicode completion) \
      --zsh <(SHELL=/bin/zsh $out/bin/pixicode completion)
  '';

  nativeInstallCheckInputs = [
    versionCheckHook
    writableTmpDirAsHomeHook
  ];
  doInstallCheck = true;
  versionCheckKeepEnvironment = [ "HOME" "PIXICODE_DISABLE_MODELS_FETCH" ];
  versionCheckProgramArg = "--version";

  passthru = {
    jsonschema = "${placeholder "out"}/share/pixicode/schema.json";
  };

  meta = {
    description = "The open source coding agent";
    homepage = "https://pixibox.ai/";
    license = lib.licenses.mit;
    mainProgram = "pixicode";
    inherit (node_modules.meta) platforms;
  };
})
