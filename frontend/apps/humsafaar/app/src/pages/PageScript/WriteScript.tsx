import { useState, type ReactNode } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { CheckIcon, CopyIcon, DownloadIcon, FilmIcon, ImageIcon, ImagePlusIcon, ImagesIcon, MicIcon, ArrowDownIcon, ArrowUpIcon, Trash2Icon, PencilIcon, SparklesIcon, Volume2Icon, WandSparklesIcon, XIcon } from "lucide-react";
import { GenerationProvider } from "@storyteller/api-enums";
import { downloadFileFromUrl, GenerateScript, GroqVoiceoverGenerate, GROQ_VOICES, TranscribeAudio } from "@storyteller/tauri-api";
import { Button } from "@storyteller/ui-button";
import { usePromptImageStore } from "@storyteller/ui-promptbox";
import { splitShots } from "./splitShots";
import { cleanRefined, REFINE_SYSTEM_PROMPT, refineRequest } from "./refineShot";
import { afterDelete, swapped } from "./shotOrder";
import { useShotMedia } from "./useShotMedia";
import { useTabStore } from "~/pages/Stores/TabState";

const SYSTEM_PROMPT =
  "You are a screenwriter. Write the scene as a numbered list of shots, one line per shot, each a self-contained visual description that an image or video model can use as its prompt.";

const message = (err: unknown) =>
  typeof err === "string" ? err : ((err as { message?: string })?.message ?? "Something went wrong");

// What the buttons act on: the words highlighted inside this card, or the whole shot when none are.
const textFrom = (card: HTMLElement, shot: string) => {
  const selection = window.getSelection();
  const picked = selection && !selection.isCollapsed && card.contains(selection.anchorNode) && card.contains(selection.focusNode)
    ? selection.toString().trim()
    : "";
  return picked || shot;
};

const Downloadable = ({ label, onSave, children }: { label: string; onSave: () => void; children: ReactNode }) => (
  <div className="flex items-end gap-2">
    {children}
    <button aria-label={`Download ${label}`} title="Download" className="text-base-fg/60 hover:text-base-fg" onClick={onSave}>
      <DownloadIcon size={16} />
    </button>
  </div>
);

const WriteScript = () => {
  const [idea, setIdea] = useState("");
  const [shots, setShots] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [voice, setVoice] = useState<(typeof GROQ_VOICES)[number]>("hannah");
  const [speaking, setSpeaking] = useState<number | null>(null);
  const [audio, setAudio] = useState<Record<number, string>>({});
  const [transcribing, setTranscribing] = useState(false);
  const [transcript, setTranscript] = useState("");

  const { media, generateImage, generateVideos, remap, reset, imageModel, imageProvider, videoModel } = useShotMedia();
  const [startingVideos, setStartingVideos] = useState(false);
  const [startingImages, setStartingImages] = useState(false);
  const [picked, setPicked] = useState<number[]>([]);
  const [editing, setEditing] = useState<number | null>(null);
  const [draft, setDraft] = useState("");
  const [instruction, setInstruction] = useState("");
  const [refining, setRefining] = useState(false);

  const startEdit = (i: number) => {
    setEditing(i);
    setDraft(shots[i]);
    setInstruction("");
  };
  const saveEdit = () => {
    if (editing === null) return;
    const text = draft.trim();
    if (text) setShots((all) => all.map((s, i) => (i === editing ? text : s)));
    setEditing(null);
  };
  // The model's rewrite goes into the editor, so the user can accept it or keep tweaking.
  const refine = async () => {
    setRefining(true);
    await run(async () => {
      setDraft(cleanRefined(await GenerateScript({ prompt: refineRequest(draft, instruction), system_prompt: REFINE_SYSTEM_PROMPT })));
      setInstruction("");
    });
    setRefining(false);
  };

  // Pictures, clips and voice follow their shot when it moves; they are keyed by position.
  const locked = editing !== null || speaking !== null || startingImages || startingVideos;
  const reshuffle = (to: (i: number) => number | null, next: string[]) => {
    remap(to);
    setAudio((a) => Object.fromEntries(Object.entries(a).flatMap(([k, v]) => {
      const n = to(Number(k));
      return n === null ? [] : [[n, v]];
    })));
    setPicked((p) => p.flatMap((i) => to(i) ?? []));
    setShots(next);
  };
  const moveShot = (i: number, by: -1 | 1) => {
    const j = i + by;
    const next = [...shots];
    [next[i], next[j]] = [next[j], next[i]];
    reshuffle(swapped(i, j), next);
  };
  const deleteShot = (i: number) => reshuffle(afterDelete(i), shots.filter((_, k) => k !== i));

  const anyPicturePending = Object.values(media).some((m) => m.imagePending);
  const makeImages = async (which = shots.map((_, i) => i)) => {
    const todo = which.filter((i) => !media[i]?.imagePending);
    if (!todo.length) return;
    // Local generation is free; anything else is billed per picture by the user's own provider.
    if (imageProvider !== GenerationProvider.ComfyUi) {
      const ok = await confirm(`Make ${todo.length} picture${todo.length === 1 ? "" : "s"} with ${imageModel?.selectorName ?? "the image model"}? Your provider bills each one.`, { title: "Generate images" });
      if (!ok) return;
    }
    setStartingImages(true);
    for (const i of todo) {
      // Stop at the first refusal (no key, no credits): the rest would only repeat it.
      if (!(await run(() => generateImage(i, shots[i])))) break;
    }
    setStartingImages(false);
  };
  const picturesReady = Object.values(media).filter((m) => m.image).length;

  const makeVideos = async (which?: number[]) => {
    setStartingVideos(true);
    await run(async () => {
      const skipped = await generateVideos(shots, which);
      if (skipped) setError(`${skipped} shot${skipped === 1 ? " has" : "s have"} no picture yet, so no video was started for ${skipped === 1 ? "it" : "them"}.`);
    });
    setStartingVideos(false);
  };

  const save = (url: string) => run(() => downloadFileFromUrl(url));

  const run = async (work: () => Promise<void>): Promise<boolean> => {
    setError(null);
    try {
      await work();
      return true;
    } catch (err) {
      setError(message(err));
      return false;
    }
  };

  const write = async () => {
    setBusy(true);
    await run(async () => {
      setShots(splitShots(await GenerateScript({ prompt: idea, system_prompt: SYSTEM_PROMPT })));
      setPicked([]);
      reset();
    });
    setBusy(false);
  };

  const speak = async (i: number, text: string) => {
    setSpeaking(i);
    await run(async () => {
      const spoken = await GroqVoiceoverGenerate(text, voice);
      setAudio((a) => ({ ...a, [i]: spoken.payload.url }));
    });
    setSpeaking(null);
  };

  const transcribe = async () => {
    const picked = await open({ multiple: false, filters: [{ name: "Audio", extensions: ["wav", "mp3", "m4a", "ogg", "flac", "webm", "mp4"] }] });
    if (typeof picked !== "string") return;
    setTranscribing(true);
    await run(async () => setTranscript(await TranscribeAudio(picked)));
    setTranscribing(false);
  };

  const sendToImage = (text: string) => {
    usePromptImageStore.getState().setPrompt(text);
    useTabStore.getState().setActiveTab("IMAGE");
  };

  // Buttons must not steal the highlight, or there would be nothing left to act on.
  const keepSelection = (e: React.MouseEvent) => e.preventDefault();

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-4 p-6 pt-20">
      <textarea
        className="h-32 w-full rounded-lg border border-white/10 bg-transparent p-3"
        placeholder="Describe the scene or story you want broken into shots"
        value={idea}
        onChange={(e) => setIdea(e.target.value)}
      />
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="primary" icon={SparklesIcon} loading={busy} disabled={busy || !idea.trim()} onClick={write}>
          Write shots
        </Button>
        <Button variant="action" icon={MicIcon} loading={transcribing} disabled={transcribing} onClick={() => run(transcribe)}>
          Transcribe a recording
        </Button>
        <label className="ml-auto flex items-center gap-2 text-base-fg/70">
          Voice
          <select
            className="rounded-md border border-white/10 bg-transparent p-1"
            value={voice}
            onChange={(e) => setVoice(e.target.value as (typeof GROQ_VOICES)[number])}
          >
            {GROQ_VOICES.map((v) => (
              <option key={v} value={v} className="text-black">{v}</option>
            ))}
          </select>
        </label>
      </div>
      {error && <p className="text-red-400">{error}</p>}
      {transcript && (
        <div className="flex flex-col gap-2 rounded-lg border border-white/10 p-3">
          <textarea className="h-24 w-full bg-transparent" value={transcript} onChange={(e) => setTranscript(e.target.value)} />
          <Button variant="action" onClick={() => setIdea(transcript)}>Use as story idea</Button>
        </div>
      )}
      <p className="text-sm text-base-fg/50">Highlight words in a shot to copy, speak or send just those words.</p>
      {shots.length > 0 && (
        <div className="flex flex-wrap items-center gap-3">
          <Button variant="primary" icon={ImagesIcon} loading={startingImages} disabled={startingImages || anyPicturePending || editing !== null} onClick={() => makeImages()}>
            Generate all images
          </Button>
          <Button variant="primary" icon={FilmIcon} loading={startingVideos} disabled={startingVideos || picturesReady === 0} onClick={() => makeVideos()}>
            Generate all videos
          </Button>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              aria-label="Select all shots"
              checked={picked.length === shots.length}
              onChange={(e) => setPicked(e.target.checked ? shots.map((_, i) => i) : [])}
            />
            Select all
          </label>
          {picked.length > 0 && (
            <Button variant="primary" icon={ImagesIcon} disabled={startingImages || editing !== null} onClick={() => makeImages(picked)}>
              Generate images for {picked.length} selected
            </Button>
          )}
          {picked.length > 0 && (
            <Button variant="primary" icon={FilmIcon} disabled={startingVideos} onClick={() => makeVideos(picked)}>
              Generate videos for {picked.length} selected
            </Button>
          )}
          <span className="text-sm text-base-fg/60">
            {picturesReady === 0
              ? "Edit the shots first if you like, then make pictures. Each picture becomes the first frame of its clip."
              : `${picturesReady} of ${shots.length} shots have a picture. Uses ${videoModel?.selectorName ?? "the video model from Create Video"}.`}
          </span>
        </div>
      )}
      {shots.map((shot, i) => (
        <div
          key={i}
          className="flex flex-wrap items-start gap-3 rounded-lg border border-white/10 p-3"
        >
          <input
            type="checkbox"
            aria-label={`Select shot ${i + 1}`}
            className="mt-1 shrink-0"
            checked={picked.includes(i)}
            onChange={(e) => setPicked((p) => (e.target.checked ? [...p, i] : p.filter((k) => k !== i)))}
          />
          <span className="w-6 shrink-0 text-base-fg/50">{i + 1}</span>
          {editing === i ? (
            <div className="flex min-w-0 grow flex-col gap-2">
              <textarea
                className="h-24 w-full rounded-md border border-white/10 bg-transparent p-2"
                aria-label={`Edit shot ${i + 1}`}
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
              />
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className="min-w-0 grow rounded-md border border-white/10 bg-transparent p-2"
                  aria-label={`Ask AI to change shot ${i + 1}`}
                  placeholder="Ask AI to change it, e.g. make it night, closer angle"
                  value={instruction}
                  onChange={(e) => setInstruction(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && instruction.trim() && !refining && refine()}
                />
                <Button variant="action" icon={WandSparklesIcon} loading={refining} disabled={refining || !instruction.trim() || !draft.trim()} onClick={refine}>
                  Refine with AI
                </Button>
              </div>
              <div className="flex gap-2">
                <Button variant="primary" icon={CheckIcon} disabled={!draft.trim()} onClick={saveEdit}>Save</Button>
                <Button variant="action" icon={XIcon} onClick={() => setEditing(null)}>Cancel</Button>
              </div>
            </div>
          ) : (
            <p className="min-w-0 grow select-text" data-shot>{shot}</p>
          )}
          <button
            aria-label={`Move shot ${i + 1} up`}
            title="Move up"
            disabled={locked || i === 0}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onClick={() => moveShot(i, -1)}
          >
            <ArrowUpIcon size={16} />
          </button>
          <button
            aria-label={`Move shot ${i + 1} down`}
            title="Move down"
            disabled={locked || i === shots.length - 1}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onClick={() => moveShot(i, 1)}
          >
            <ArrowDownIcon size={16} />
          </button>
          <button
            aria-label={`Delete shot ${i + 1}`}
            title="Delete shot"
            disabled={locked}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onClick={() => deleteShot(i)}
          >
            <Trash2Icon size={16} />
          </button>
          <button
            aria-label={`Edit shot ${i + 1}`}
            title="Edit or refine with AI"
            disabled={editing !== null}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onMouseDown={keepSelection}
            onClick={() => startEdit(i)}
          >
            <PencilIcon size={16} />
          </button>
          <button
            aria-label={`Speak shot ${i + 1}`}
            title="Speak"
            disabled={speaking !== null}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onMouseDown={keepSelection}
            onClick={(e) => speak(i, textFrom(e.currentTarget.parentElement as HTMLElement, shot))}
          >
            <Volume2Icon size={16} />
          </button>
          <button
            aria-label={`Generate image for shot ${i + 1}`}
            title={imageModel ? `Generate image with ${imageModel.selectorName}` : "No image model available"}
            disabled={media[i]?.imagePending}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onMouseDown={keepSelection}
            onClick={(e) => run(() => generateImage(i, textFrom(e.currentTarget.parentElement as HTMLElement, shot)))}
          >
            <ImagePlusIcon size={16} />
          </button>
          <button
            aria-label={`Generate video for shot ${i + 1}`}
            title={media[i]?.image ? `Generate video with ${videoModel?.selectorName ?? "the video model"}` : "Make a picture first"}
            disabled={!media[i]?.image || media[i]?.videoPending}
            className="shrink-0 text-base-fg/60 hover:text-base-fg disabled:opacity-40"
            onMouseDown={keepSelection}
            onClick={() => makeVideos([i])}
          >
            <FilmIcon size={16} />
          </button>
          <button
            aria-label={`Send shot ${i + 1} to Create Image`}
            title="Send to Create Image"
            className="shrink-0 text-base-fg/60 hover:text-base-fg"
            onMouseDown={keepSelection}
            onClick={(e) => sendToImage(textFrom(e.currentTarget.parentElement as HTMLElement, shot))}
          >
            <ImageIcon size={16} />
          </button>
          <button
            aria-label={`Copy shot ${i + 1}`}
            title="Copy"
            className="shrink-0 text-base-fg/60 hover:text-base-fg"
            onMouseDown={keepSelection}
            onClick={(e) => navigator.clipboard.writeText(textFrom(e.currentTarget.parentElement as HTMLElement, shot))}
          >
            <CopyIcon size={16} />
          </button>
          {speaking === i && <span className="basis-full pl-9 text-sm text-base-fg/60">Speaking…</span>}
          {media[i]?.imagePending && <span className="basis-full pl-9 text-sm text-base-fg/60">Making the picture…</span>}
          {media[i]?.image && <Downloadable label={`picture for shot ${i + 1}`} onSave={() => save(media[i].image!.url)}><img className="max-h-56 rounded-md" src={media[i].image!.url} alt={`Shot ${i + 1}`} /></Downloadable>}
          {media[i]?.videoPending && <span className="basis-full pl-9 text-sm text-base-fg/60">Making the video…</span>}
          {media[i]?.video && <Downloadable label={`video for shot ${i + 1}`} onSave={() => save(media[i].video!.url)}><video className="max-h-56 rounded-md" controls src={media[i].video!.url} /></Downloadable>}
          {audio[i] && <Downloadable label={`voice for shot ${i + 1}`} onSave={() => save(audio[i])}><audio controls src={audio[i]} /></Downloadable>}
        </div>
      ))}
    </div>
  );
};

export default WriteScript;
