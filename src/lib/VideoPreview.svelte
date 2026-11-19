<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte';
  import { api } from './api';

  let { path }: { path: string } = $props();
  let source = $state('');
  let converting = $state(false);
  let converted = $state(false);
  let error = $state('');
  let request: AbortController;

  $effect(() => {
    const file = path;
    const controller = request = new AbortController();
    source = ''; converting = false; converted = false; error = '';
    void api.videoSource(file).then((url) => {
      if (!controller.signal.aborted) source = url;
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) error = reason instanceof Error ? reason.message : String(reason);
    });
    return () => controller.abort();
  });

  async function failed(event: Event) {
    const video = event.currentTarget as HTMLVideoElement;
    const { signal } = request;
    const mediaError = video.error;
    if (signal.aborted || !video.isConnected || converting || (mediaError && mediaError.code === mediaError.MEDIA_ERR_ABORTED)) return;
    if (converted) {
      error = '当前系统无法播放转换后的视频。';
      return;
    }

    const format = video.canPlayType('video/mp4; codecs="avc1.42E01F, mp4a.40.2"') ? 'mp4' : 'webm';
    source = '';
    converting = true;
    try {
      const fallback = await api.compatibleVideoSource(path, format, signal);
      if (signal.aborted) return;
      if (fallback) {
        converted = true;
        source = fallback;
      } else {
        error = '视频可能已损坏，或当前系统不支持此格式。';
      }
    } catch (reason) {
      if (!signal.aborted) error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      if (!signal.aborted) converting = false;
    }
  }

  async function startPlayback(event: Event) {
    const video = event.currentTarget as HTMLVideoElement;
    try {
      await video.play();
    } catch (reason) {
      // Only retry when autoplay policy blocks sound.
      if (!(reason instanceof Error) || reason.name !== 'NotAllowedError' || !video.isConnected || !video.paused) return;
      video.muted = true;
      try { await video.play(); } catch { /* Playback remains available through the controls. */ }
    }
  }

  function releaseMedia(video: HTMLVideoElement) {
    return {
      destroy() {
        video.pause();
        video.removeAttribute('src');
        video.load();
      },
    };
  }
</script>

{#if error}
  <div class="file-preview-message" role="status">无法预览：{error}</div>
{:else if source}
  <video class="video-preview" use:releaseMedia controls autoplay playsinline preload="metadata" src={source} onloadedmetadata={startPlayback} onerror={failed}><track kind="captions" /></video>
{:else}
  <div class="file-preview-message" role="status"><LoaderCircle size={22} class="spin" />{converting ? '正在转换视频…' : '正在加载视频…'}</div>
{/if}
