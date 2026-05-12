import { invoke, RuntimeStyleSheet, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import { convertFileSrc } from "@tauri-apps/api/core";
import { lazyRune } from "../../utils";

const players = lazyRune(() => invoke(SeelenCommand.GetMediaSessions));
subscribe(SeelenEvent.MediaSessions, players.setByPayload);
await players.init();

const player = $derived(players.value.find((p) => p.default));
const thumbnailSrc = $derived(player?.thumbnail ? convertFileSrc(player.thumbnail) : null);

let fetchingThumbnail = $state(false);

$effect.root(() => {
  $effect(() => {
    if (player) {
      const sheet = new RuntimeStyleSheet("@runtime/media-player-meta");
      sheet.addVariable("--media-player-title", `"${player.title}"`);
      sheet.addVariable("--media-player-artist", `"${player.author}"`);
      sheet.applyToDocument();
    }
  });

  let fetchId = 0;

  $effect(() => {
    if (!thumbnailSrc) {
      fetchingThumbnail = false;
      return;
    }

    const currentId = ++fetchId;
    fetchingThumbnail = true;
    fetch(thumbnailSrc)
      .then(async (res) => {
        const blob = await res.blob();
        return new Promise<string>((resolve, reject) => {
          const reader = new FileReader();
          reader.onloadend = () => resolve(reader.result as string);
          reader.onerror = reject;
          reader.readAsDataURL(blob);
        });
      })
      .then((data) => {
        if (currentId !== fetchId) return;
        const sheet = new RuntimeStyleSheet("@runtime/media-player-thumbnail");
        sheet.addVariable("--media-player-thumbnail", `url("${data}")`);
        sheet.applyToDocument();
      })
      .finally(() => {
        if (currentId !== fetchId) return;
        fetchingThumbnail = false;
      });
  });
});

class WallpaperState {
  get player() {
    return player;
  }
  get fetchingThumbnail() {
    return fetchingThumbnail;
  }
}

export default new WallpaperState();
