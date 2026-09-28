import { createEffect, createMemo } from "solid-js";
import { createStore } from "solid-js/store";
import { createTimeline } from "animejs";
import { useTicker } from "@diffusionstudio/jsx";

/** @inspect color path="Brand/Cream" */
const cream = "#F6F0E6";

/** @inspect color path="Brand/Graphite" */
const graphite = "#292926";

/** @inspect color path="Brand/Orange" */
const orange = "#FF9A62";

/** @inspect color path="Brand/Yellow" */
const yellow = "#F4CD58";

/** @inspect color path="Brand/Blue" */
const blue = "#8BC7F8";

/** @inspect color path="Brand/Mint" */
const mint = "#8EDBB6";

/** @inspect color path="Brand/Lavender" */
const lavender = "#B9A8EB";

/** @inspect text path="Brand/Tagline" */
const tagline = "Terminal work, simplified.";

const DURATION = 20;

type Pose = { x: number; y: number; rotation: number; scale: number };
type PoseStop = Pose & { time: number };

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));
const mix = (from: number, to: number, amount: number) => from + (to - from) * amount;
const ease = (value: number) => {
  const x = clamp01(value);
  return x * x * (3 - 2 * x);
};
const spring = (time: number, start: number, duration: number, damping = 7.5, frequency = 12.5) => {
  const x = (time - start) / duration;
  if (x <= 0) return 0;
  if (x >= 1) return 1;
  return 1 - Math.exp(-damping * x) * Math.cos(frequency * x);
};
const revealY = (time: number, start: number, distance = 70) => distance * (1 - spring(time, start, 0.62, 8.2, 10.8));
const typed = (text: string, time: number, start: number, cps = 18) => text.slice(0, Math.floor(Math.max(0, time - start) * cps));
const beat = (time: number, at: number, length = 0.28) => {
  const x = clamp01((time - at) / length);
  return Math.sin(x * Math.PI) * (time >= at && time <= at + length ? 1 : 0);
};

const samplePose = (time: number, stops: PoseStop[]): Pose => {
  if (time <= stops[0].time) return stops[0];
  for (let index = 1; index < stops.length; index += 1) {
    const previous = stops[index - 1];
    const next = stops[index];
    if (time <= next.time) {
      const raw = (time - previous.time) / (next.time - previous.time);
      const amount = index === 1 ? spring(time, previous.time, next.time - previous.time) : ease(raw);
      return {
        x: mix(previous.x, next.x, amount),
        y: mix(previous.y, next.y, amount),
        rotation: mix(previous.rotation, next.rotation, amount),
        scale: mix(previous.scale, next.scale, amount),
      };
    }
  }
  return stops[stops.length - 1];
};

const paths = {
  key: [
    { time: 0, x: 210, y: -180, rotation: -24, scale: 0.78 },
    { time: 1.45, x: 244, y: 194, rotation: -8, scale: 1 },
    { time: 4.8, x: 270, y: 180, rotation: -4, scale: 1.05 },
    { time: 8.8, x: 172, y: 704, rotation: 9, scale: 0.92 },
    { time: 12.8, x: 238, y: 742, rotation: -8, scale: 1.02 },
    { time: 16.8, x: 248, y: 812, rotation: 5, scale: 0.72 },
    { time: 20, x: 266, y: 802, rotation: 9, scale: 0.72 },
  ],
  folder: [
    { time: 0, x: 1520, y: -210, rotation: 19, scale: 0.76 },
    { time: 1.72, x: 1548, y: 190, rotation: 7, scale: 1 },
    { time: 4.8, x: 1510, y: 176, rotation: 4, scale: 1.02 },
    { time: 8.8, x: 1180, y: 480, rotation: -5, scale: 0.92 },
    { time: 12.8, x: 1600, y: 748, rotation: 7, scale: 0.95 },
    { time: 16.8, x: 1570, y: 825, rotation: -5, scale: 0.68 },
    { time: 20, x: 1548, y: 812, rotation: -8, scale: 0.68 },
  ],
  magnifier: [
    { time: 0, x: 1740, y: -150, rotation: 35, scale: 0.72 },
    { time: 1.86, x: 1650, y: 690, rotation: 12, scale: 1 },
    { time: 4.8, x: 1618, y: 686, rotation: 5, scale: 1.02 },
    { time: 8.8, x: 1415, y: 480, rotation: -11, scale: 0.9 },
    { time: 12.8, x: 1630, y: 270, rotation: 10, scale: 0.98 },
    { time: 16.8, x: 1650, y: 250, rotation: 18, scale: 0.7 },
    { time: 20, x: 1638, y: 266, rotation: 13, scale: 0.7 },
  ],
  prompt: [
    { time: 0, x: 520, y: -160, rotation: -18, scale: 0.72 },
    { time: 1.62, x: 510, y: 800, rotation: -5, scale: 1 },
    { time: 4.8, x: 550, y: 822, rotation: 2, scale: 1.04 },
    { time: 8.8, x: 1065, y: 480, rotation: 3, scale: 0.82 },
    { time: 12.8, x: 305, y: 280, rotation: -8, scale: 0.92 },
    { time: 16.8, x: 320, y: 260, rotation: -2, scale: 0.68 },
    { time: 20, x: 332, y: 272, rotation: 3, scale: 0.68 },
  ],
};

const chipPaths = [
  [
    { time: 0, x: 360, y: -120, rotation: -16, scale: 0.75 },
    { time: 2.55, x: 360, y: 375, rotation: -8, scale: 1 },
    { time: 4.8, x: 342, y: 380, rotation: -4, scale: 1 },
    { time: 8.8, x: 1160, y: 330, rotation: 1, scale: 0.86 },
    { time: 12.8, x: 250, y: 500, rotation: -5, scale: 0.88 },
    { time: 16.8, x: 350, y: 870, rotation: 4, scale: 0.62 },
    { time: 20, x: 370, y: 862, rotation: 7, scale: 0.62 },
  ],
  [
    { time: 0, x: 780, y: -160, rotation: 20, scale: 0.74 },
    { time: 2.72, x: 720, y: 182, rotation: 7, scale: 1 },
    { time: 4.8, x: 710, y: 172, rotation: 3, scale: 1 },
    { time: 8.8, x: 1395, y: 330, rotation: -2, scale: 0.86 },
    { time: 12.8, x: 500, y: 826, rotation: 4, scale: 0.9 },
    { time: 16.8, x: 820, y: 860, rotation: -4, scale: 0.6 },
    { time: 20, x: 805, y: 850, rotation: -7, scale: 0.6 },
  ],
  [
    { time: 0, x: 1120, y: -130, rotation: -15, scale: 0.78 },
    { time: 2.86, x: 1225, y: 178, rotation: -5, scale: 1 },
    { time: 4.8, x: 1240, y: 172, rotation: -2, scale: 1 },
    { time: 8.8, x: 1588, y: 330, rotation: 4, scale: 0.86 },
    { time: 12.8, x: 1438, y: 840, rotation: -3, scale: 0.88 },
    { time: 16.8, x: 1210, y: 860, rotation: 4, scale: 0.6 },
    { time: 20, x: 1225, y: 850, rotation: 7, scale: 0.6 },
  ],
  [
    { time: 0, x: 1380, y: -120, rotation: 18, scale: 0.76 },
    { time: 3.02, x: 1450, y: 500, rotation: 6, scale: 1 },
    { time: 4.8, x: 1460, y: 515, rotation: 2, scale: 1 },
    { time: 8.8, x: 1700, y: 330, rotation: -5, scale: 0.82 },
    { time: 12.8, x: 1680, y: 520, rotation: 5, scale: 0.86 },
    { time: 16.8, x: 1480, y: 860, rotation: -5, scale: 0.58 },
    { time: 20, x: 1465, y: 850, rotation: -8, scale: 0.58 },
  ],
];

function poseStyle(pose: Pose, time: number, phase: number) {
  const driftX = Math.sin(time * 0.72 + phase) * 5;
  const driftY = Math.sin(time * 0.88 + phase * 1.7) * 7;
  return `translate3d(${pose.x + driftX}px,${pose.y + driftY}px,0) rotate(${pose.rotation}deg) scale(${pose.scale})`;
}

function ToyChip(props: { label: string; color: string; pose: Pose; time: number; phase: number }) {
  return (
    <div class="toy-chip" style={{ transform: poseStyle(props.pose, props.time, props.phase), background: props.color }}>
      <span>{props.label}</span>
    </div>
  );
}

export default function TFinderLightPromo() {
  const { time } = useTicker();
  const camera = { x: -8, y: 4 };
  const [v, setV] = createStore({ camera: { ...camera } });

  const motion = createTimeline({ autoplay: false })
    .add(camera, { x: 14, y: -7, duration: DURATION * 1000, ease: "inOutSine" }, 0);

  createEffect(() => {
    motion.seek(time() * 1000);
    setV("camera", { ...camera });
  });

  const terminal = createMemo(() => {
    const t = time();
    if (t < 2.15) {
      const p = spring(t, 0.58, 1.48, 7.7, 12.2);
      return { x: 960, y: 532, width: mix(24, 1080, p), height: mix(22, 520, p), radius: mix(11, 44, p) };
    }
    if (t < 5) {
      const p = ease((t - 2.15) / 2.85);
      return { x: mix(960, 900, p), y: mix(532, 526, p), width: mix(1080, 1160, p), height: mix(520, 530, p), radius: 44 };
    }
    if (t < 9) {
      const p = spring(t, 5, 1.05, 8.5, 11.6);
      return { x: mix(900, 600, p), y: mix(526, 515, p), width: mix(1160, 820, p), height: mix(530, 474, p), radius: mix(44, 40, p) };
    }
    if (t < 13) {
      const p = spring(t, 9, 1.05, 8.4, 11.8);
      const reaction = beat(t, 10.15) * 6 + beat(t, 11.35) * 6 + beat(t, 12.45) * 6;
      return { x: mix(600, 960, p), y: mix(515, 520, p) - reaction, width: mix(820, 1280, p), height: mix(474, 560, p), radius: mix(40, 48, p) };
    }
    if (t < 16.65) {
      const p = spring(t, 13, 3.65, 7.8, 9.3);
      return { x: mix(960, 514, p), y: mix(520, 520, p), width: mix(1280, 154, p), height: mix(560, 108, p), radius: mix(48, 32, p) };
    }
    return { x: 514, y: 520 + Math.sin(t * 0.9) * 3, width: 154, height: 108, radius: 32 };
  });

  const stage = createMemo(() => {
    const t = time();
    if (t < 2) return 0;
    if (t < 5) return 1;
    if (t < 9) return 2;
    if (t < 13) return 3;
    if (t < 16.65) return 4;
    return 5;
  });

  const cursorVisible = createMemo(() => Math.floor(time() * 2.2) % 2 === 0);
  const keyPose = createMemo(() => samplePose(time(), paths.key));
  const folderPose = createMemo(() => samplePose(time(), paths.folder));
  const magnifierPose = createMemo(() => samplePose(time(), paths.magnifier));
  const promptPose = createMemo(() => samplePose(time(), paths.prompt));
  const chips = createMemo(() => chipPaths.map((path) => samplePose(time(), path)));
  const heroStep = createMemo(() => Math.max(0, Math.min(2, Math.floor((time() - 9.35) / 1.12))));
  const finalReveal = createMemo(() => spring(time(), 15.25, 1.25, 8.4, 10.6));
  const finalPrompt = createMemo(() => typed("$ tf", time(), 17.25, 7));

  return (
    <stage background="#D8D1C6" camera={[0.3, 0, 0, 0.3, 85, 150]} id="xw06hy">
      <scene id="tfinder-light-promo" name="TFinder — light tactile 4K launch film" width={1920} height={1080} fill={cream} workarea={[0, DURATION]} active>
        <html id="tfinder-world" width={1920} height={1080} end={DURATION}>
          <div class="world">
            <style>{`
              * { box-sizing: border-box; }
              .world {
                --cream:${cream}; --graphite:${graphite}; --orange:${orange}; --yellow:${yellow};
                --blue:${blue}; --mint:${mint}; --lavender:${lavender};
                position:relative; width:100%; height:100%; overflow:hidden; color:var(--graphite);
                background:var(--cream); font-family:"SF Pro Display","Helvetica Neue",Inter,system-ui,sans-serif;
              }
              .ambient { position:absolute; border-radius:999px; }
              .ambient.a { width:720px;height:720px;left:-250px;top:-320px;background:rgba(255,154,98,.11); }
              .ambient.b { width:620px;height:620px;right:-180px;bottom:-300px;background:rgba(139,199,248,.13); }
              .ambient.c { width:410px;height:410px;right:210px;top:-250px;background:rgba(185,168,235,.10); }
              .paper-grid { position:absolute; inset:0; background-image:linear-gradient(rgba(41,41,38,.035) 1px,transparent 1px),linear-gradient(90deg,rgba(41,41,38,.035) 1px,transparent 1px); background-size:80px 80px; }
              .camera { position:absolute; inset:0; perspective:1400px; transform-style:preserve-3d; }
              .floor-shadow { position:absolute; left:430px; top:810px; width:1080px; height:110px; border-radius:50%; background:rgba(82,68,52,.09); filter:blur(24px); }
              .terminal { position:absolute; background:#FFFDF9; border:1px solid rgba(41,41,38,.11); overflow:hidden; box-shadow:0 34px 80px rgba(84,67,47,.16),0 8px 22px rgba(84,67,47,.10),inset 0 1px 0 #fff; }
              .terminal::after { content:""; position:absolute; inset:0; border-radius:inherit; box-shadow:inset 0 0 0 1px rgba(255,255,255,.72); pointer-events:none; }
              .terminal-bar { height:64px; display:flex; align-items:center; gap:9px; padding:0 26px; background:#FBF7F0; border-bottom:1px solid rgba(41,41,38,.08); }
              .traffic { width:12px;height:12px;border-radius:50%;box-shadow:inset 0 -2px 3px rgba(41,41,38,.12); }
              .traffic.one{background:var(--orange)} .traffic.two{background:var(--yellow)} .traffic.three{background:var(--mint)}
              .terminal-label { margin-left:12px; color:#9B958C; font:600 14px "SFMono-Regular",Menlo,monospace; letter-spacing:.07em; }
              .terminal-body { padding:34px 38px; font-family:"SFMono-Regular","SF Mono",Menlo,monospace; color:#383733; }
              .command { font-size:31px; font-weight:650; letter-spacing:-.02em; }
              .dollar { color:#817B72; margin-right:13px; }
              .cursor { display:inline-block; width:13px;height:31px;margin-left:5px;border-radius:5px;background:var(--orange);vertical-align:-5px; }
              .subline { margin-top:24px; color:#8F8980; font-size:18px; line-height:1.55; }
              .result { margin-top:24px; padding:20px 22px; border:1px solid rgba(41,41,38,.09); background:#F8F4ED; border-radius:20px; font-size:18px; display:flex; align-items:center; gap:16px; }
              .result-check { width:34px;height:34px;border-radius:50%;display:grid;place-items:center;background:var(--mint);font-weight:900; }
              .hero-history { display:flex; flex-direction:column; gap:13px; margin-top:22px; }
              .hero-line { min-height:72px;padding:17px 20px;border-radius:18px;background:#F8F4ED;border:1px solid rgba(41,41,38,.08);display:grid;grid-template-columns:1fr auto;align-items:center;gap:20px;font-size:17px; }
              .hero-line strong { font-weight:650; }
              .status-pill { padding:8px 12px;border-radius:999px;background:var(--mint);font:700 12px "SFMono-Regular",Menlo,monospace;letter-spacing:.05em; }
              .logo-symbol { position:absolute;inset:0;display:grid;place-items:center;font:760 58px "SFMono-Regular",Menlo,monospace;color:var(--graphite);letter-spacing:-.11em; }
              .headline { position:absolute; z-index:10; letter-spacing:-.055em; font-weight:720; line-height:.92; }
              .headline .accent { color:#F0783D; }
              .mask { overflow:hidden; }
              .intro-headline { left:116px; bottom:118px; display:flex;align-items:flex-end;gap:22px; }
              .intro-headline .small { font-size:58px; }
              .intro-headline .fast { font-size:102px; color:#F0783D; }
              .search-headline { left:100px; bottom:70px; display:flex;gap:16px;font-size:69px; }
              .search-headline span:nth-child(2){color:#458CA9}.search-headline span:nth-child(3){color:#5D9D78}
              .hero-headline { left:112px; top:118px; display:flex;gap:20px;font-size:70px; }
              .hero-headline .less { color:#8B6CC9; }
              .toy { position:absolute; left:0; top:0; transform-origin:center; transform-style:preserve-3d; }
              .keycap { width:136px;height:112px;border-radius:28px;background:#FFF9EA;border:1px solid rgba(41,41,38,.12);box-shadow:0 18px 28px rgba(84,67,47,.15),inset 0 -9px 0 rgba(222,207,174,.55),inset 0 2px 0 #fff;display:grid;place-items:center;font:750 36px "SFMono-Regular",Menlo,monospace; }
              .folder { width:155px;height:118px;border-radius:22px 28px 28px 28px;background:var(--yellow);box-shadow:0 22px 36px rgba(128,94,31,.16),inset 0 2px 0 rgba(255,255,255,.6); }
              .folder::before { content:"";position:absolute;left:12px;top:-20px;width:72px;height:31px;border-radius:16px 18px 0 0;background:#E7B93D; }
              .folder::after { content:"";position:absolute;left:19px;right:19px;top:27px;height:5px;border-radius:5px;background:rgba(255,255,255,.45); }
              .magnifier { width:108px;height:108px;border:18px solid var(--blue);border-radius:50%;box-shadow:0 18px 32px rgba(71,121,158,.18),inset 0 3px 8px rgba(255,255,255,.9);background:rgba(255,255,255,.45); }
              .magnifier::after { content:"";position:absolute;width:72px;height:24px;border-radius:14px;background:#65A9DC;right:-58px;bottom:-31px;transform:rotate(44deg);box-shadow:0 10px 18px rgba(71,121,158,.16); }
              .prompt-toy { width:148px;height:94px;border-radius:28px;background:var(--lavender);display:grid;place-items:center;box-shadow:0 20px 34px rgba(104,83,151,.17),inset 0 2px 0 rgba(255,255,255,.55);font:760 37px "SFMono-Regular",Menlo,monospace; }
              .toy-chip { position:absolute;left:0;top:0;min-width:148px;height:66px;padding:0 24px;border-radius:22px;border:1px solid rgba(41,41,38,.09);display:grid;place-items:center;box-shadow:0 16px 28px rgba(84,67,47,.13),inset 0 2px 0 rgba(255,255,255,.55);font:760 15px "SFMono-Regular",Menlo,monospace;letter-spacing:.08em;transform-origin:center; }
              .pipeline { position:absolute;left:1060px;top:284px;width:760px;height:430px; }
              .pipeline-rail { position:absolute;left:65px;right:70px;top:214px;height:8px;border-radius:999px;background:rgba(41,41,38,.09); }
              .pipeline-rail::after { content:"";position:absolute;left:0;top:0;height:100%;width:var(--progress);border-radius:inherit;background:#75B993; }
              .pipeline-card { position:absolute;width:178px;height:118px;border-radius:28px;background:#FFFDF9;border:1px solid rgba(41,41,38,.1);box-shadow:0 20px 36px rgba(84,67,47,.14);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;text-align:center; }
              .pipeline-card small { font:700 12px "SFMono-Regular",Menlo,monospace;color:#928C84;letter-spacing:.08em; }
              .pipeline-card b { font-size:19px; }
              .pipe-a{left:0;top:150px}.pipe-b{left:290px;top:150px}.pipe-c{left:580px;top:150px}
              .mini-file { width:46px;height:52px;border-radius:9px;background:#FFD5BD;border:1px solid rgba(41,41,38,.08);position:relative; }
              .mini-file::after { content:"";position:absolute;left:11px;right:11px;top:18px;height:4px;border-radius:4px;background:rgba(41,41,38,.28);box-shadow:0 10px 0 rgba(41,41,38,.18); }
              .arrow { position:absolute;top:194px;font-size:33px;color:#A9A39A; }
              .arrow.one{left:225px}.arrow.two{left:515px}
              .brand-lockup { position:absolute;left:650px;top:385px;width:930px;height:310px;display:flex;flex-direction:column;justify-content:center; }
              .brand-name { font-size:132px;font-weight:760;letter-spacing:-.075em;line-height:.85; }
              .brand-tagline { margin-top:30px;font-size:34px;color:#706C65;letter-spacing:-.025em; }
              .final-prompt { margin-top:32px;font:700 24px "SFMono-Regular",Menlo,monospace;color:#706C65; }
              .final-cursor { display:inline-block;width:10px;height:25px;border-radius:4px;background:var(--orange);vertical-align:-4px;margin-left:5px; }
              .brand-mask { overflow:hidden;padding-bottom:8px; }
              .micro-label { position:absolute;left:72px;top:62px;font:750 16px "SFMono-Regular",Menlo,monospace;letter-spacing:.1em;color:#777169; }
              .micro-label b { color:#F0783D; }
              .corner-note { position:absolute;right:72px;top:62px;font:650 13px "SFMono-Regular",Menlo,monospace;letter-spacing:.11em;color:#979087; }
            `}</style>

            <div class="ambient a" />
            <div class="ambient b" />
            <div class="ambient c" />
            <div class="paper-grid" />

            <div class="micro-label"><b>TF</b> / TERMINAL COMPANION</div>
            <div class="corner-note">FAST · TACTILE · LOCAL</div>

            <div class="camera" style={{ transform: `translate3d(${v.camera.x}px,${v.camera.y}px,0)` }}>
              <div class="floor-shadow" style={{ transform: `translateY(${Math.sin(time() * 0.65) * 5}px)` }} />

              <div class="toy" style={{ transform: poseStyle(keyPose(), time(), 0.2) }}><div class="keycap">⌘</div></div>
              <div class="toy" style={{ transform: poseStyle(folderPose(), time(), 1.1) }}><div class="folder" /></div>
              <div class="toy" style={{ transform: poseStyle(magnifierPose(), time(), 2.2) }}><div class="magnifier" /></div>
              <div class="toy" style={{ transform: poseStyle(promptPose(), time(), 3.1) }}><div class="prompt-toy">{">_"}</div></div>

              <ToyChip label="SEARCH" color={orange} pose={chips()[0]} time={time()} phase={0.8} />
              <ToyChip label="FIND" color={blue} pose={chips()[1]} time={time()} phase={1.8} />
              <ToyChip label="INSPECT" color={mint} pose={chips()[2]} time={time()} phase={2.8} />
              <ToyChip label="RUN" color={lavender} pose={chips()[3]} time={time()} phase={3.8} />

              {stage() === 2 && (
                <div class="pipeline" style={{ "--progress": `${Math.round(ease((time() - 5.25) / 2.8) * 100)}%` }}>
                  <div class="pipeline-rail" />
                  <div class="pipeline-card pipe-a" style={{ transform: `translateY(${revealY(time(), 5.18, 52)}px)` }}>
                    <div class="mini-file" /><small>INPUT</small><b>project/</b>
                  </div>
                  <div class="arrow one">→</div>
                  <div class="pipeline-card pipe-b" style={{ transform: `translateY(${revealY(time(), 5.62, 52)}px)` }}>
                    <div style="font-size:38px">⌕</div><small>MATCH</small><b>*.tsx</b>
                  </div>
                  <div class="arrow two">→</div>
                  <div class="pipeline-card pipe-c" style={{ transform: `translateY(${revealY(time(), 6.08, 52)}px)` }}>
                    <div class="result-check">✓</div><small>RESULT</small><b>48 files</b>
                  </div>
                </div>
              )}

              <div
                class="terminal"
                style={{
                  left: `${terminal().x - terminal().width / 2}px`,
                  top: `${terminal().y - terminal().height / 2}px`,
                  width: `${terminal().width}px`,
                  height: `${terminal().height}px`,
                  "border-radius": `${terminal().radius}px`,
                  "box-shadow": `0 ${34 + beat(time(), 4.1) * 8}px ${80 + beat(time(), 4.1) * 18}px rgba(84,67,47,.16), 0 8px 22px rgba(84,67,47,.10), inset 0 1px 0 #fff`,
                }}
              >
                {terminal().width > 390 && stage() < 4 && (
                  <div class="terminal-bar">
                    <i class="traffic one" /><i class="traffic two" /><i class="traffic three" />
                    <span class="terminal-label">TFinder · zsh</span>
                  </div>
                )}

                {stage() === 0 && <div class="logo-symbol" style={{ visibility: cursorVisible() ? "visible" : "hidden", color: orange }}>_</div>}

                {stage() === 1 && (
                  <div class="terminal-body">
                    <div class="command"><span class="dollar">$</span>{typed("tf", time(), 2.28, 8)}<span class="cursor" style={{ visibility: cursorVisible() ? "visible" : "hidden", transform: `scaleX(${1 + beat(time(), 2.65) * .22})` }} /></div>
                    {time() > 3.08 && <div class="subline" style={{ transform: `translateY(${revealY(time(), 3.08, 28)}px)` }}>Ready. Ask naturally — TFinder resolves the action.</div>}
                    {time() > 3.68 && <div class="result" style={{ transform: `translateY(${revealY(time(), 3.68, 34)}px)` }}><span class="result-check">✓</span><span><b>40 typed actions</b><br />No generated shell.</span></div>}
                  </div>
                )}

                {stage() === 2 && (
                  <div class="terminal-body">
                    <div class="command"><span class="dollar">$</span>{typed("tf find every .tsx file", time(), 5.18, 22)}<span class="cursor" style={{ visibility: cursorVisible() ? "visible" : "hidden" }} /></div>
                    {time() > 6.45 && <div class="subline" style={{ transform: `translateY(${revealY(time(), 6.45, 26)}px)` }}>Scanning this project…</div>}
                    {time() > 7.1 && <div class="result" style={{ transform: `translateY(${revealY(time(), 7.1, 30)}px)` }}><span class="result-check">✓</span><span><b>48 files found</b><br />src · tests · scripts</span></div>}
                  </div>
                )}

                {stage() === 3 && (
                  <div class="terminal-body">
                    <div class="command"><span class="dollar">$</span>{heroStep() === 0 ? "tf inspect PID 46272" : heroStep() === 1 ? "tf find .tsx files" : "tf plan disable bluetooth"}<span class="cursor" style={{ visibility: cursorVisible() ? "visible" : "hidden" }} /></div>
                    <div class="hero-history">
                      <div class="hero-line" style={{ transform: `translateY(${revealY(time(), 9.5, 32)}px)` }}><strong>PID 46272 · Microsoft Edge</strong><span class="status-pill">INSPECTED</span></div>
                      {heroStep() >= 1 && <div class="hero-line" style={{ transform: `translateY(${revealY(time(), 10.55, 32)}px)` }}><strong>48 TypeScript files</strong><span class="status-pill">FOUND</span></div>}
                      {heroStep() >= 2 && <div class="hero-line" style={{ transform: `translateY(${revealY(time(), 11.68, 32)}px)` }}><strong>Turn Bluetooth off · preview only</strong><span class="status-pill">SAFE</span></div>}
                    </div>
                  </div>
                )}

                {stage() >= 4 && <div class="logo-symbol" style={{ "font-size": `${Math.max(42, Math.min(92, terminal().width * .36))}px` }}>{">_"}</div>}
              </div>

              {stage() === 1 && (
                <div class="headline intro-headline">
                  <div class="mask"><div class="small" style={{ transform: `translateY(${revealY(time(), 2.18, 74)}px)` }}>Your terminal.</div></div>
                  <div class="mask"><div class="fast" style={{ transform: `translateY(${revealY(time(), 2.56, 110)}px)` }}>Faster.</div></div>
                </div>
              )}

              {stage() === 2 && (
                <div class="headline search-headline">
                  <div class="mask"><span style={{ display: "block", transform: `translateY(${revealY(time(), 5.2, 82)}px)` }}>Find it.</span></div>
                  <div class="mask"><span style={{ display: "block", transform: `translateY(${revealY(time(), 5.68, 82)}px)` }}>Do it.</span></div>
                  <div class="mask"><span style={{ display: "block", transform: `translateY(${revealY(time(), 6.16, 82)}px)` }}>Keep moving.</span></div>
                </div>
              )}

              {stage() === 3 && (
                <div class="headline hero-headline">
                  <div class="mask"><span style={{ display: "block", transform: `translateY(${revealY(time(), 9.12, 82)}px)` }}>One command.</span></div>
                  <div class="mask"><span class="less" style={{ display: "block", transform: `translateY(${revealY(time(), 9.48, 82)}px)` }}>Less friction.</span></div>
                </div>
              )}

              {time() >= 15.25 && (
                <div class="brand-lockup">
                  <div class="brand-mask"><div class="brand-name" style={{ transform: `translateY(${(1 - finalReveal()) * 145}px)` }}>TFinder</div></div>
                  <div class="brand-mask"><div class="brand-tagline" style={{ transform: `translateY(${(1 - spring(time(), 15.62, .95, 8.5, 10.2)) * 54}px)` }}>{tagline}</div></div>
                  {stage() === 5 && <div class="final-prompt">{finalPrompt()}<span class="final-cursor" style={{ visibility: cursorVisible() ? "visible" : "hidden", transform: `translateY(${Math.sin(time() * 1.1) * 1.5}px)` }} /></div>}
                </div>
              )}
            </div>
          </div>
        </html>
        <audio id="soundtrack" name="Original 120 BPM tactile soundtrack" src="tfinder-light-bed-final.wav" start={0} end={DURATION} volume={0} />
      </scene>
    </stage>
  );
}
