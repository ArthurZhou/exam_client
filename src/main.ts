import { invoke } from "@tauri-apps/api/core";

function initOSD() {
  if (document.getElementById('exam-osd-root')) return;

  const container = document.createElement('div');
  container.id = 'exam-osd-root';
  container.style.cssText = 'position:fixed;z-index:2147483647;pointer-events:auto;';
  document.body.appendChild(container);

  const shadow = container.attachShadow({ mode: 'open' });
  const style = document.createElement('style');
  
  // 采用 Neo-Brutalism 风格：硬边框、高对比、等宽字体
  style.textContent = `
    :host { all: initial; }
    .osd-wrapper {
      position: fixed; top: 20px; right: 25px; width: 220px;
      background: #ffffff; color: #1a1a1a;
      padding: 15px; border: 3px solid #1a1a1a;
      font-family: 'sarasa';
      box-shadow: 5px 5px 0px #1a1a1a; /* 经典的硬阴影 */
    }
    @media (prefers-color-scheme: dark) {
      .osd-wrapper { background: #121212; color: #eeeeee; border-color: #eeeeee; box-shadow: 8px 8px 0px #444444; }
    }

    .toolbar { 
      display: grid; grid-template-columns: repeat(4, 1fr); gap: 0; 
      margin-bottom: 15px; border: 2px solid currentColor;
    }
    .nav-btn { 
      background: transparent; border: none; color: inherit; 
      padding: 2px 0; cursor: pointer; font-size: 16px; font-weight: 900;
      border-right: 2px solid currentColor;
    }
    .nav-btn:last-child { border-right: none; }
    .nav-btn:hover { background: rgba(128,128,128,0.1); }
    .nav-btn:active { background: currentColor; color: var(--bg-inverse); }

    .header { 
      font-weight: 900; font-size: 12px; text-transform: uppercase;
      display: flex; align-items: center; justify-content: space-between;
      letter-spacing: 0.5px;
    }
    .status-group { display: flex; align-items: center; }
    .dot { width: 10px; height: 10px; background: #00ff00; border: 2px solid #1a1a1a; margin-right: 8px; }
    
    .menu-toggle { 
      font-size: 11px; color: inherit; cursor: pointer; 
      text-decoration: underline; font-weight: bold; 
    }

    #exit-dropdown { display: none; margin-top: 15px; }
    #exit-dropdown.show { display: block; }

    input#exit-password { 
      width: 100%; padding: 10px; margin-bottom: 10px;
      border: 2px solid currentColor; background: transparent; color: inherit;
      font-family: inherit; box-sizing: border-box; outline: none;
    }

    button.action-btn { 
      width: 100%; padding: 12px; cursor: pointer; font-family: inherit;
      font-weight: 900; text-transform: uppercase; border: 2px solid currentColor;
      transition: transform 0.1s;
    }
    .btn-main { background: #1a1a1a; color: #ffffff; }
    @media (prefers-color-scheme: dark) { .btn-main { background: #eeeeee; color: #121212; } }
    
    .btn-cancel { background: transparent; color: inherit; margin-top: 8px; }
    
    .action-btn:active { transform: translate(2px, 2px); }

    .msg { font-size: 11px; color: #ff3e3e; margin-top: 8px; font-weight: bold; text-transform: uppercase; }
  `;

  const panel = document.createElement('div');
  panel.className = 'osd-wrapper';
  panel.innerHTML = `
    <div class="toolbar">
      <button class="nav-btn" id="nav-home">⌂</button>
      <button class="nav-btn" id="nav-back">\<</button>
      <button class="nav-btn" id="nav-forward">\></button>
      <button class="nav-btn" id="nav-reload">↻</button>
    </div>
    <div class="header">
      <div class="status-group"><span class="dot"></span>反作弊运行中</div>
      <span class="menu-toggle" id="toggle-menu">管理</span>
    </div>
    <div id="exit-dropdown">
      <div id="exit-section">
        <button class="action-btn btn-main" id="btn-exit">关闭考试窗口</button>
      </div>
      <div id="exit-msg" class="msg"></div>
    </div>
  `;

  shadow.appendChild(style);
  shadow.appendChild(panel);

  const getFrame = () => document.getElementById('exam-frame') as HTMLIFrameElement | null;

  // 导航逻辑
  shadow.getElementById('nav-home')?.addEventListener('click', async () => {
    const cfg = await invoke('get_config').catch(() => ({})) as any;
    const frame = getFrame();
    if (cfg?.exam_url && frame) frame.src = cfg.exam_url;
  });
  shadow.getElementById('nav-back')?.addEventListener('click', () => {
    try { getFrame()?.contentWindow?.history.back(); } catch(e) { console.error(e); }
  });
  shadow.getElementById('nav-forward')?.addEventListener('click', () => {
    try { getFrame()?.contentWindow?.history.forward(); } catch(e) { console.error(e); }
  });
  shadow.getElementById('nav-reload')?.addEventListener('click', () => {
    const f = getFrame();
    if (f) f.src = f.src;
  });

  shadow.getElementById('toggle-menu')?.addEventListener('click', () => {
    shadow.getElementById('exit-dropdown')?.classList.toggle('show');
  });

  const attachExitHandler = () => {
    shadow.getElementById('btn-exit')?.addEventListener('click', () => {
      const section = shadow.getElementById('exit-section');
      if (!section) return;
      section.innerHTML = `
        <input type="password" id="exit-password" placeholder="管理密码" autofocus />
        <button class="action-btn btn-main" id="btn-confirm-exit">确认退出</button>
        <button class="action-btn btn-cancel" id="btn-cancel-exit">取消</button>
      `;
      shadow.getElementById('btn-confirm-exit')?.addEventListener('click', async () => {
        const pwd = (shadow.getElementById('exit-password') as HTMLInputElement).value;
        const msg = shadow.getElementById('exit-msg');
        if (!pwd) return;
        try {
          await invoke('request_exit', { password: pwd });
        } catch (e) {
          if (msg) msg.textContent = `! ERROR: ${e}`;
        }
      });
      shadow.getElementById('btn-cancel-exit')?.addEventListener('click', () => {
        section.innerHTML = `<button class="action-btn btn-main" id="btn-exit">关闭考试窗口</button>`;
        attachExitHandler();
      });
    });
  };
  attachExitHandler();
}

// 统一执行逻辑保持不变...
if (document.readyState === 'complete') {
  initOSD();
} else {
  window.addEventListener('load', initOSD);
}

(async () => {
  try {
    const cfg = await invoke('get_config') as any;
    if (cfg?.exam_url) {
      const frame = document.getElementById('exam-frame') as HTMLIFrameElement;
      if (frame) frame.src = cfg.exam_url;
    }
  } catch (e) {
    console.error('Config load failed', e);
  }
})();