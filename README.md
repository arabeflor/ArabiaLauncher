# ArabiaLauncher

ArabiaLauncher is a Minecraft launcher project with two separate editions: **Portable** and **Online**. They differ in how they store and prepare game files—not in whether Minecraft can be played with other people.

> **ArabiaLauncher is not affiliated with, endorsed by, or officially connected to Mojang Studios or Microsoft.** Minecraft is a trademark of Mojang Studios. This is an independent project.

## Two editions, one important distinction

| Edition | How it prepares the game | Java | Multiplayer |
| --- | --- | --- | --- |
| **Portable** | Keeps the launcher and prepared game environment together in a folder. | Uses the Java 8 runtime included with the package. | **Can be used for multiplayer.** “Portable” describes how the files are stored; it does not mean the game is limited to offline or single-player play. |
| **Online** | Downloads and prepares game files on the computer. | Uses a Java runtime installed or configured on the computer. | Multiplayer depends on the profile, connection, server, and its authentication requirements. |

**Portable does not mean offline-only.** You can use the Portable edition for LAN or internet multiplayer when you have a connection and the server supports the profile and authentication mode available to you. The name **Online Edition** refers to its download-and-install workflow; it does not mean that multiplayer is exclusive to that edition.

## Portable Edition

The Portable edition is designed to keep the launcher and its prepared game environment together, making the folder easier to move or back up.

- Includes Minecraft 1.12.2, Forge 14.23.5.2859, and Java 8 x64 in the prepared environment.
- Keeps game data, worlds, mods, and settings alongside the launcher, in the `mcdata` folder.
- Does not require a traditional Minecraft or Java installation for the included environment.
- Provides launcher screens for the game, mods, library, settings, and news.
- Supports enabling and disabling mods through the launcher.
- Offers English, Brazilian Portuguese, Spanish, French, and Russian interface languages.
- Supports LAN and internet multiplayer, subject to network connectivity and the server’s profile and authentication requirements.

To use it, extract the Portable package, keep `ArabiaLauncher.exe` together with `mcdata`, and open the launcher. The included Java runtime is located at `mcdata\\java\\jre-legacy\\bin\\java.exe`.

## Online Edition

The Online edition downloads and prepares Minecraft files on the computer instead of relying on a complete game environment stored alongside the launcher.

- Browses the official Minecraft version catalog.
- Provides search and filters for release, alpha, beta, and snapshot versions.
- Lets you select a Java runtime detected on the computer or configure a custom Java executable.
- Downloads required game files when you press **Play**, then starts the game when installation finishes.
- Verifies downloaded files against hashes and sizes supplied by Mojang when available.
- Provides a repair action in Settings to reinstall the selected version.
- Displays installation progress in the launcher.
- Saves the selected version and launcher settings locally.
- Supports English, Brazilian Portuguese, Spanish, French, and Russian.

The Online edition currently uses a local offline player profile and does **not** provide Microsoft sign-in. As a result, multiplayer server access depends on the server accepting that profile and authentication mode; servers requiring an authenticated Microsoft account may not be accessible through this edition.

An internet connection is needed to fetch the catalog and download files that are not already installed. Game files are stored locally after downloading.

## Multiplayer and the edition names

Both editions run Minecraft, which can support multiplayer. The difference is **how the launcher prepares and stores the game**:

- **Portable** keeps the prepared environment together in its folder.
- **Online** downloads and installs the selected game files on the computer.

Neither the word “Portable” nor the name “Online Edition” alone determines whether a server will let you join. Multiplayer also depends on having a suitable connection and satisfying the server’s version, profile, and authentication requirements.

## Building from source

The project is written in Rust using `eframe`/`egui`. With the Rust stable toolchain installed, run these commands from the relevant edition’s project directory:

```powershell
cargo run --release
```

To create a release executable:

```powershell
cargo build --release
```

The exact files required alongside the executable depend on the edition and its distribution package.

## Disclaimer

ArabiaLauncher is an independent, community-made project. It is not affiliated with, endorsed by, or sponsored by Mojang Studios or Microsoft. Minecraft and related names are the property of their respective owners.
