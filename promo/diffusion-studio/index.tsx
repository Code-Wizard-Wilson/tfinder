import { createEffect, createMemo } from "solid-js";
import { createStore } from "solid-js/store";
import { createTimeline } from "animejs";
import { useTicker } from "@diffusionstudio/jsx";

/** @inspect color path="Brand/Accent" */
const accent = "#78F55A";

/** @inspect color path="Brand/Background" */
const background = "#050605";

/** @inspect color path="Brand/Surface" */
const surface = "#111311";

/** @inspect text path="End card/CTA" */
const finalCta = "OPEN SOURCE · LINK IN POST";

const DURATION = 25;

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));
const reveal = (time: number, at: number, duration = 0.28) => clamp01((time - at) / duration);
const easeOut = (value: number) => 1 - Math.pow(1 - clamp01(value), 3);
const easeIn = (value: number) => Math.pow(clamp01(value), 2);
const shotY = (time: number, start: number, end: number, enterY = 34, exitY = -20) => {
  const incoming = enterY * (1 - easeOut((time - start) / 0.4));
  const outgoing = exitY * easeIn((time - (end - 0.2)) / 0.2);
  return incoming + outgoing;
};

const processRows = [
  ["46272", "10.1%", "2.1%", "349.3 MiB", "Microsoft Edge"],
  ["63222", "30.3%", "1.4%", "223.4 MiB", "opencode"],
  ["377", "28.6%", "0.7%", "120.7 MiB", "WindowServer"],
  ["667", "16.6%", "0.8%", "132.0 MiB", "ghostty"],
];

const languageRows = [
  ["RU", "почему мак тормозит", "TOP_PROCESSES"],
  ["EN", "find every .tsx file here", "FIND_FILES"],
  ["ES", "qué proceso usa más memoria", "TOP_PROCESSES"],
];

const motion = {
  scan: { x: -60 },
  endRule: { width: 0 },
};

export default function TerFinderPromo() {
  const { time } = useTicker();
  const [v, setV] = createStore({
    scan: { ...motion.scan },
    endRule: { ...motion.endRule },
  });

  const timeline = createTimeline({ autoplay: false })
    .add(motion.scan, { x: 1980, ease: "linear", duration: 25000 }, 0)
    .add(motion.endRule, { width: { from: 0, to: 292 }, ease: "cubicBezier(0,0.65,0.51,0.99)", duration: 700 }, 20600);

  createEffect(() => {
    timeline.seek(time() * 1000);
    setV("scan", { ...motion.scan });
    setV("endRule", { ...motion.endRule });
  });

  const diagTyped = createMemo(() => {
    const value = "почему мак тормозит";
    return value.slice(0, Math.floor(Math.max(0, time() - 3.45) * 20));
  });

  const pidTyped = createMemo(() => {
    const value = "что за PID 46272";
    return value.slice(0, Math.floor(Math.max(0, time() - 7.8) * 22));
  });

  const safetyTyped = createMemo(() => {
    const value = "plan выключи блютуз";
    return value.slice(0, Math.floor(Math.max(0, time() - 15.8) * 22));
  });

  const cursor = createMemo(() => (Math.floor(time() * 2.4) % 2 === 0 ? 1 : 0));
  const diagResult = createMemo(() => reveal(time(), 4.62));
  const pidResult = createMemo(() => reveal(time(), 8.82));
  const safetyResult = createMemo(() => reveal(time(), 17.08));
  const currentShot = createMemo(() => {
    const t = time();
    if (t < 3) return 0;
    if (t < 7.4) return 1;
    if (t < 11.3) return 2;
    if (t < 15.3) return 3;
    if (t < 20.6) return 4;
    return 5;
  });

  return (
    <stage background="#161616" camera={[0.35, 0, 0, 0.35, 66.07, 132.77]} id="l2369q">
      <scene id="terfinder-promo" name="TerFinder — X launch film" width={1920} height={1080} fill={background} workarea={[0, DURATION]} active>
        <html id="promo-ui" width={1920} height={1080} end={DURATION} selected>
          <div class="film">
            <style>{`
              * { box-sizing: border-box; }
              .film {
                --accent: ${accent}; --surface: ${surface};
                position: relative; width: 100%; height: 100%; overflow: hidden;
                background: ${background}; color: #F8F8F8;
                font-family: "SF Pro Display", "Helvetica Neue", Inter, system-ui, sans-serif;
              }
              .film::before {
                content: ""; position: absolute; inset: 0;
                background-image: linear-gradient(rgba(255,255,255,.027) 1px, transparent 1px),
                                  linear-gradient(90deg, rgba(255,255,255,.027) 1px, transparent 1px);
                background-size: 96px 96px;
              }
              .grain {
                position: absolute; inset: 0; pointer-events: none;
                background-image: repeating-linear-gradient(0deg, rgba(255,255,255,.015) 0, rgba(255,255,255,.015) 1px, transparent 1px, transparent 4px);
              }
              .scan { position: absolute; top: 0; bottom: 0; width: 1px; background: rgba(120,245,90,.32); box-shadow: 0 0 26px rgba(120,245,90,.18); }
              .topbar { position: absolute; top: 50px; left: 64px; right: 64px; display: flex; justify-content: space-between; align-items: center; z-index: 5; }
              .brand { display: flex; align-items: center; gap: 14px; font-weight: 650; letter-spacing: -.02em; font-size: 24px; }
              .brand-mark { width: 38px; height: 38px; border: 1px solid var(--accent); display: grid; place-items: center; color: var(--accent); font: 700 18px "SFMono-Regular", Menlo, monospace; }
              .meta { color: #8D918D; font: 500 15px "SFMono-Regular", Menlo, monospace; letter-spacing: .12em; }
              .shot { position: absolute; inset: 0; padding: 150px 112px 92px; }
              .eyebrow { color: var(--accent); font: 600 19px "SFMono-Regular", Menlo, monospace; letter-spacing: .12em; text-transform: uppercase; margin-bottom: 28px; }
              h1, h2, p { margin: 0; }
              .hook h1 { max-width: 1480px; font-size: 112px; line-height: .94; letter-spacing: -.064em; font-weight: 650; }
              .hook .qualifier { position: absolute; left: 116px; bottom: 104px; font-size: 32px; color: #9DA19D; letter-spacing: -.02em; }
              .hook .command-fragment { position: absolute; right: 116px; bottom: 102px; color: var(--accent); font: 560 25px "SFMono-Regular", Menlo, monospace; }
              .scene-heading { display: flex; align-items: end; justify-content: space-between; margin-bottom: 46px; }
              .scene-heading h2 { font-size: 64px; line-height: 1; letter-spacing: -.045em; font-weight: 620; }
              .scene-heading p { color: #929692; font: 500 17px "SFMono-Regular", Menlo, monospace; letter-spacing: .08em; }
              .terminal { width: 100%; border: 1px solid rgba(255,255,255,.12); background: #0A0C0A; box-shadow: 0 38px 100px rgba(0,0,0,.42); font-family: "SFMono-Regular", "SF Mono", Menlo, monospace; }
              .terminal-head { height: 58px; display: flex; align-items: center; padding: 0 22px; border-bottom: 1px solid rgba(255,255,255,.09); color: #818581; font-size: 14px; letter-spacing: .08em; }
              .lights { display: flex; gap: 9px; margin-right: 18px; }
              .lights i { display: block; width: 11px; height: 11px; border-radius: 50%; background: #3B3E3B; }
              .terminal-body { min-height: 470px; padding: 34px 38px 32px; font-size: 24px; line-height: 1.55; }
              .prompt { color: #D9DDD9; white-space: pre; }
              .prompt b { color: var(--accent); font-weight: 650; }
              .cursor { color: var(--accent); }
              .response { margin-top: 28px; color: #F1F3F1; }
              .response-label { color: #8B908B; font-size: 17px; margin-bottom: 14px; }
              .table { color: #C9CDC9; font-size: 19px; line-height: 1.75; }
              .table-row { display: grid; grid-template-columns: 100px 100px 100px 180px 1fr; border-top: 1px solid rgba(255,255,255,.065); padding: 8px 0; }
              .table-row.head { color: #727672; border-top: 0; font-size: 14px; letter-spacing: .08em; }
              .table-row.focus { color: #F8F8F8; }
              .table-row.focus span:first-child { color: var(--accent); }
              .pid-answer { display: flex; gap: 28px; align-items: center; padding-top: 34px; }
              .pid-icon { width: 84px; height: 84px; border: 1px solid var(--accent); color: var(--accent); display: grid; place-items: center; font-size: 34px; font-weight: 700; }
              .pid-title { font-size: 37px; letter-spacing: -.025em; }
              .pid-path { margin-top: 8px; color: #8D918D; font-size: 16px; }
              .language-copy { width: 720px; }
              .language-copy h2 { font-size: 88px; line-height: .96; letter-spacing: -.055em; font-weight: 640; }
              .language-copy p { margin-top: 28px; color: #9B9F9B; font-size: 28px; line-height: 1.4; }
              .language-layout { height: 100%; display: flex; align-items: center; justify-content: space-between; gap: 100px; }
              .language-stack { width: 800px; display: flex; flex-direction: column; gap: 16px; }
              .language-row { min-height: 128px; padding: 22px 26px; border: 1px solid rgba(255,255,255,.11); background: rgba(17,19,17,.94); display: grid; grid-template-columns: 72px 1fr auto; align-items: center; gap: 22px; }
              .lang { color: var(--accent); font: 700 16px "SFMono-Regular", Menlo, monospace; }
              .phrase { font: 500 23px "SFMono-Regular", Menlo, monospace; }
              .intent { color: #8E928E; font: 600 13px "SFMono-Regular", Menlo, monospace; letter-spacing: .06em; border: 1px solid rgba(255,255,255,.1); padding: 9px 12px; }
              .safety .terminal-body { min-height: 500px; }
              .plan-grid { margin-top: 30px; display: grid; grid-template-columns: 190px 1fr; gap: 0; border-top: 1px solid rgba(255,255,255,.08); }
              .plan-grid div { min-height: 66px; display: flex; align-items: center; border-bottom: 1px solid rgba(255,255,255,.08); }
              .plan-key { color: #7E827E; font-size: 15px; letter-spacing: .07em; }
              .plan-value { color: #EBEEEB; font-size: 19px; }
              .safe-note { margin-top: 26px; color: var(--accent); font-size: 19px; display: flex; align-items: center; gap: 12px; }
              .check { width: 28px; height: 28px; border: 1px solid var(--accent); display: grid; place-items: center; font-size: 16px; }
              .end-card { display: flex; flex-direction: column; justify-content: center; }
              .end-lockup { display: flex; align-items: center; gap: 48px; }
              .end-mark { color: var(--accent); font: 720 184px/.8 "SFMono-Regular", Menlo, monospace; letter-spacing: -.12em; }
              .end-name { font-size: 112px; line-height: .88; letter-spacing: -.065em; font-weight: 650; }
              .end-tag { margin-top: 28px; color: #A0A4A0; font-size: 31px; line-height: 1.3; }
              .end-rule { height: 2px; width: 0; background: var(--accent); margin: 58px 0 32px; }
              .end-proof { display: flex; align-items: center; justify-content: space-between; }
              .stats { color: #D2D5D2; font: 520 17px "SFMono-Regular", Menlo, monospace; letter-spacing: .035em; }
              .cta { color: var(--accent); font: 650 16px "SFMono-Regular", Menlo, monospace; letter-spacing: .12em; }
            `}</style>

            <div class="grain" />
            <div class="scan" style={{ transform: `translateX(${v.scan.x}px)` }} />

            <div class="topbar">
              <div class="brand"><span class="brand-mark">tf</span><span>TerFinder</span></div>
              <div class="meta">LOCAL · TYPED ACTIONS · macOS</div>
            </div>
            {currentShot() === 0 && <div class="shot hook" style={{ transform: `translateY(${shotY(time(), 0, 3)}px)` }}>
              <div class="eyebrow">YOUR TERMINAL, WITHOUT THE MANUAL</div>
              <h1>Say what you need.<br />Not the command.</h1>
              <p class="qualifier">Natural language in. Safe macOS actions out.</p>
              <div class="command-fragment">{"> tf _"}</div>
            </div>}

            {currentShot() === 1 && <div class="shot diagnostic" style={{ transform: `translateY(${shotY(time(), 3, 7.4, 30)}px)` }}>
              <div class="scene-heading">
                <h2>Ask the real question.</h2>
                <p>01 / DIAGNOSE</p>
              </div>
              <div class="terminal" style={{ transform: `translateY(${42 * (1 - easeOut((time() - 3.2) / 0.5))}px)` }}>
                <div class="terminal-head"><span class="lights"><i /><i /><i /></span>~/projects — zsh</div>
                <div class="terminal-body">
                  <div class="prompt"><b>{"> tf"}</b> {diagTyped()}<span class="cursor" style={{ opacity: cursor() }}>▌</span></div>
                  {time() >= 4.62 && <div class="response" style={{ transform: `translateY(${(1 - diagResult()) * 14}px)` }}>
                    <div class="response-label">Top processes by CPU</div>
                    <div class="table">
                      <div class="table-row head"><span>PID</span><span>CPU</span><span>MEM</span><span>RSS</span><span>COMMAND</span></div>
                      {processRows.map((row, index) => (
                        <div class={`table-row ${index === 0 ? "focus" : ""}`}>
                          {row.map((cell) => <span>{cell}</span>)}
                        </div>
                      ))}
                    </div>
                  </div>}
                </div>
              </div>
            </div>}

            {currentShot() === 2 && <div class="shot pid" style={{ transform: `translateY(${shotY(time(), 7.4, 11.3, 30)}px)` }}>
              <div class="scene-heading">
                <h2>Context, not keyword roulette.</h2>
                <p>02 / RESOLVE</p>
              </div>
              <div class="terminal" style={{ transform: `translateY(${42 * (1 - easeOut((time() - 7.5) / 0.5))}px)` }}>
                <div class="terminal-head"><span class="lights"><i /><i /><i /></span>~/projects — zsh</div>
                <div class="terminal-body">
                  <div class="prompt"><b>{"> tf"}</b> {pidTyped()}<span class="cursor" style={{ opacity: cursor() }}>▌</span></div>
                  {time() >= 8.82 && <div class="response pid-answer" style={{ transform: `translateY(${(1 - pidResult()) * 16}px)` }}>
                    <div class="pid-icon">E</div>
                    <div>
                      <div class="pid-title">PID 46272 · Microsoft Edge</div>
                      <div class="pid-path">/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge</div>
                    </div>
                  </div>}
                </div>
              </div>
            </div>}

            {currentShot() === 3 && <div class="shot languages" style={{ transform: `translateY(${shotY(time(), 11.3, 15.3, 32)}px)` }}>
              <div class="language-layout">
                <div class="language-copy">
                  <div class="eyebrow">ONE ACTION SYSTEM</div>
                  <h2>17 languages.<br />Mixed phrasing.</h2>
                  <p>The same typed intent underneath. No generated shell.</p>
                </div>
                <div class="language-stack">
                  {languageRows.map((row, index) => {
                    const local = () => reveal(time(), 11.45 + index * 0.38, 0.32);
                    return (
                      <div class="language-row" style={{ transform: `translateY(${(1 - local()) * 34}px)` }}>
                        <span class="lang">{row[0]}</span>
                        <span class="phrase">{row[1]}</span>
                        <span class="intent">{row[2]}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>}

            {currentShot() === 4 && <div class="shot safety" style={{ transform: `translateY(${shotY(time(), 15.3, 20.6, 30)}px)` }}>
              <div class="scene-heading">
                <h2>Preview the dangerous part.</h2>
                <p>03 / TRUST</p>
              </div>
              <div class="terminal" style={{ transform: `translateY(${42 * (1 - easeOut((time() - 15.4) / 0.5))}px)` }}>
                <div class="terminal-head"><span class="lights"><i /><i /><i /></span>~/projects — zsh</div>
                <div class="terminal-body">
                  <div class="prompt"><b>{"> tf"}</b> {safetyTyped()}<span class="cursor" style={{ opacity: cursor() }}>▌</span></div>
                  {time() >= 17.08 && <div class="response" style={{ transform: `translateY(${(1 - safetyResult()) * 14}px)` }}>
                    <div class="plan-grid">
                      <div class="plan-key">INTENT</div><div class="plan-value">SetBluetoothPower</div>
                      <div class="plan-key">RISK</div><div class="plan-value">Destructive</div>
                      <div class="plan-key">PLANNED ACTION</div><div class="plan-value">Turn Bluetooth off</div>
                    </div>
                    <div class="safe-note"><span class="check">✓</span>No changes made.</div>
                  </div>}
                </div>
              </div>
            </div>}

            {currentShot() === 5 && <div class="shot end-card" style={{ transform: `translateY(${34 * (1 - easeOut((time() - 20.6) / 0.5))}px)` }}>
              <div class="end-lockup">
                <div class="end-mark">{">_"}</div>
                <div>
                  <div class="end-name">TerFinder</div>
                  <div class="end-tag">Say what you need.<br />Get a safe, typed action.</div>
                </div>
              </div>
              <div class="end-rule" style={{ width: `${v.endRule.width}px` }} />
              <div class="end-proof">
                <div class="stats">v0.6.0 · 7,192 SCENARIOS · 36,976 CHECKS · 0 FAILURES</div>
                <div class="cta">{finalCta}</div>
              </div>
            </div>}
          </div>
        </html>
        <audio id="soundtrack" name="Original minimal electronic bed" src="terfinder-bed.wav" start={0} end={DURATION} volume={0} />
      </scene>
    </stage>
  );
}
