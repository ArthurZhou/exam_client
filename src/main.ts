import { invoke } from "@tauri-apps/api/core";

function initOSD() {
  if (document.getElementById('exam-osd-root')) return;

  const container = document.createElement('div');
  container.id = 'exam-osd-root';
  container.style.cssText = 'position:fixed;z-index:2147483647;pointer-events:none;'; // 容器设为 none
  document.body.appendChild(container);

  const shadow = container.attachShadow({ mode: 'open' });
  const style = document.createElement('style');

  // osd css
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
  // osd 内容
  panel.innerHTML = `
    <div class="toolbar">
      <button class="nav-btn" id="nav-home" title="主页"><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" aria-hidden="true" role="img" width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M11.336 2.253a1 1 0 0 1 1.328 0l9 8a1 1 0 0 1-1.328 1.494L20 11.45V19a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-7.55l-.336.297a1 1 0 0 1-1.328-1.494zM6 9.67V19h3v-5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v5h3V9.671l-6-5.333zM13 19v-4h-2v4z"></path></svg></button>
      <button class="nav-btn" id="nav-back" title="后退"><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" aria-hidden="true" role="img" width="24" height="24" viewBox="0 0 42 42"><path fill="currentColor" fill-rule="evenodd" d="M27.066 1L7 21.068l19.568 19.569l4.934-4.933l-14.637-14.636L32 5.933z"></path></svg></button>
      <button class="nav-btn" id="nav-forward" title="前进"><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" aria-hidden="true" role="img" width="24" height="24" viewBox="0 0 42 42"><path fill="currentColor" fill-rule="evenodd" d="M13.933 1L34 21.068L14.431 40.637l-4.933-4.933l14.638-14.636L9 5.933z"></path></svg></button>
      <button class="nav-btn" id="nav-reload" title="刷新"><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" aria-hidden="true" role="img" width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M12.793 2.293a1 1 0 0 1 1.414 0l3 3a1 1 0 0 1 0 1.414l-3 3a1 1 0 0 1-1.414-1.414L14.086 7H12.5C8.952 7 6 9.952 6 13.5S8.952 20 12.5 20s6.5-2.952 6.5-6.5a1 1 0 1 1 2 0c0 4.652-3.848 8.5-8.5 8.5S4 18.152 4 13.5S7.848 5 12.5 5h1.586l-1.293-1.293a1 1 0 0 1 0-1.414"></path></svg></button>
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

  // 导航，直接操作 window.history 和 window.location
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

function checkOriginAndRedirect() {
  // Webview2 在加载失败（如断网、DNS 错误）时，origin 会变成字符串 "null"，此时无法正常tauri invoke，必须跳转到一个本地页面来恢复退出功能
  const isInvalidOrigin = window.location.origin === "null" || window.origin === "null";

  if (isInvalidOrigin) {
    window.location.replace("http://tauri.localhost/empty.html");   // 这个仅在build构件中有效，dev无效
  }
}

if (document.readyState === 'complete') {
  initOSD();
} else {
  window.addEventListener('load', initOSD);
}
checkOriginAndRedirect();