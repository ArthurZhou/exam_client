import { invoke } from "@tauri-apps/api/core";

function initOSD() {
  if (document.getElementById('exam-osd-root')) return;

  const container = document.createElement('div');
  container.id = 'exam-osd-root';
  container.style.cssText = 'position:fixed;z-index:2147483647;pointer-events:none;'; // 容器设为 none
  document.body.appendChild(container);

  const shadow = container.attachShadow({ mode: 'open' });
  const style = document.createElement('style');

  style.textContent = `
    :host { all: initial; }
    .osd-wrapper {
      position: fixed; top: 20px; right: 25px; width: 220px;
      background: #ffffff; color: #1a1a1a;
      padding: 15px; border: 3px solid #1a1a1a;
      font-family: "Microsoft YaHei UI", "Microsoft YaHei", "SimHei", system-ui, sans-serif;
      font-weight: 900; /* 必须加粗，才能出 Neo-Brutalism 的视觉张力 */
      box-shadow: 5px 5px 0px #1a1a1a;
      pointer-events: auto; /* 仅面板恢复点击 */
      transition: linear 0.2s;
    }
    @media (prefers-color-scheme: dark) {
      .osd-wrapper { background: #121212; color: #eeeeee; border-color: #eeeeee; box-shadow: 8px 8px 0px #444444; }
    }

    .toolbar { 
      display: grid; grid-template-columns: repeat(4, 1fr); gap: 0; 
      margin-bottom: 15px; border: 0px solid currentColor;
    }
    .nav-btn { 
      background: transparent; border: none; color: inherit; 
      padding: 5px 0; cursor: pointer; font-size: 16px; font-weight: 900;
      border-right: 3px solid #444444;
    }
    .nav-btn:last-child { border-right: none; }
    .nav-btn:hover { background: rgba(128,128,128,0.1); }
    .nav-btn:active { background: #1a1a1a; color: #fff; }

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
      <button class="nav-btn" id="nav-home" title="主页">⌂</button>
      <button class="nav-btn" id="nav-back" title="后退">\<</button>
      <button class="nav-btn" id="nav-forward" title="前进">\></button>
      <button class="nav-btn" id="nav-reload" title="刷新">↻</button>
    </div>
    <div class="header">
      <div class="status-group"><span class="dot"></span>考试监控中</div>
      <span class="menu-toggle" id="toggle-menu">管理</span>
    </div>
    <div id="exit-dropdown">
      <div id="exit-section">
        <button class="action-btn btn-main" id="btn-exit">退出考试</button>
      </div>
      <div id="exit-msg" class="msg"></div>
    </div>
  `;

  shadow.appendChild(style);
  shadow.appendChild(panel);

  // 导航逻辑：直接操作 window.history 和 window.location
  shadow.getElementById('nav-home')?.addEventListener('click', async () => {
    try {
      const cfg = await invoke('get_config') as any;
      if (cfg?.exam_url) window.location.replace(cfg.exam_url);
    } catch (e) { console.error(e); }
  });

  shadow.getElementById('nav-back')?.addEventListener('click', () => {
    window.history.back();
  });

  shadow.getElementById('nav-forward')?.addEventListener('click', () => {
    window.history.forward();
  });

  shadow.getElementById('nav-reload')?.addEventListener('click', () => {
    window.location.reload();
  });

  shadow.getElementById('toggle-menu')?.addEventListener('click', () => {
    shadow.getElementById('exit-dropdown')?.classList.toggle('show');
  });

  const attachExitHandler = () => {
    shadow.getElementById('btn-exit')?.addEventListener('click', () => {
      const section = shadow.getElementById('exit-section');
      if (!section) return;
      section.innerHTML = `
        <input type="password" id="exit-password" placeholder="输入密码" />
        <button class="action-btn btn-main" id="btn-confirm-exit">确认</button>
        <button class="action-btn btn-cancel" id="btn-cancel-exit">取消</button>
      `;
      shadow.getElementById('exit-password')?.focus();

      shadow.getElementById('btn-confirm-exit')?.addEventListener('click', async () => {
        const pwd = (shadow.getElementById('exit-password') as HTMLInputElement).value;
        const msg = shadow.getElementById('exit-msg');
        if (!pwd) return;
        try {
          await invoke('request_exit', { password: pwd });
        } catch (e) {
          if (msg) msg.textContent = `! 错误: ${e}`;
        }
      });

      shadow.getElementById('btn-cancel-exit')?.addEventListener('click', () => {
        section.innerHTML = `<button class="action-btn btn-main" id="btn-exit">退出考试</button>`;
        attachExitHandler();
      });
    });
  };
  attachExitHandler();
}

if (document.readyState === 'complete') {
  initOSD();
} else {
  window.addEventListener('load', initOSD);
}