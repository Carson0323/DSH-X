(() => {
  const host = document.createElement('aside');
  host.className = 'mascot';
  host.setAttribute('aria-label', '互动看板娘');
  host.innerHTML = `
    <button class="mascot-puppet" type="button" aria-label="互动看板娘">
      <svg viewBox="0 0 1254 1254" aria-hidden="true">
        <defs>
          <clipPath id="mascot-tuft"><rect x="420" y="20" width="400" height="280"/></clipPath>
          <clipPath id="mascot-ear-left"><rect x="0" y="510" width="230" height="220"/></clipPath>
          <clipPath id="mascot-bow"><path d="M1020 610H1185V720L1140 765Q1110 782 1088 717Q1070 740 1020 740Z"/></clipPath>
          <clipPath id="mascot-ear-right"><path d="M1020 740Q1070 740 1088 717Q1110 782 1140 765L1185 720H1254V970H1020Z"/></clipPath>
          <linearGradient id="mascot-eye" x2="1" y2="1"><stop stop-color="#141a32"/><stop offset="1" stop-color="#242b49"/></linearGradient>
        </defs>
        <g transform="translate(50 50) scale(.92)">
        <g data-part="head">
          <image href="/mascot/head-v2.png" width="1254" height="1254"/>
          <g data-part="ear-left"><image href="/mascot/accessories-v2.png" width="1254" height="1254" clip-path="url(#mascot-ear-left)"/></g>
          <g data-part="ear-right"><image href="/mascot/accessories-v2.png" width="1254" height="1254" clip-path="url(#mascot-ear-right)"/></g>
          <g data-part="tuft"><image href="/mascot/accessories-v2.png" width="1254" height="1254" clip-path="url(#mascot-tuft)"/></g>
          <g data-part="bow"><image href="/mascot/accessories-v2.png" width="1254" height="1254" clip-path="url(#mascot-bow)"/></g>
          <g data-part="gaze">
            <g transform="translate(464 655) rotate(20)"><g data-part="eye-left"><ellipse rx="46" ry="80" fill="url(#mascot-eye)"/></g><path data-part="lid-left" d="M-42 8Q0 -28 42 8" fill="none" stroke="#222940" stroke-width="11" stroke-linecap="round" opacity="0"/></g>
            <g transform="translate(785 779) rotate(20)"><g data-part="eye-right"><ellipse rx="45" ry="80" fill="url(#mascot-eye)"/></g><path data-part="lid-right" d="M-41 8Q0 -27 41 8" fill="none" stroke="#222940" stroke-width="11" stroke-linecap="round" opacity="0"/></g>
          </g>
          <g data-part="blush" opacity="0" fill="#f493ac"><ellipse cx="350" cy="745" rx="54" ry="34" transform="rotate(20 350 745)"/><ellipse cx="815" cy="908" rx="52" ry="34" transform="rotate(20 815 908)"/></g>
        </g>
        </g>
      </svg>
    </button>`;
  document.body.append(host);
  const button = host.querySelector('button');
  const parts = Object.fromEntries([...host.querySelectorAll('[data-part]')].map(el => [el.dataset.part, el]));
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  let failed = false;
  let frame = 0, last = 0, clock = 0, nextBlink = 2 + Math.random() * 3;
  let blinkStart = -10, happyUntil = 0, bounce = 0;
  let targetX = 0, targetY = 0, x = 0, y = 0, tuft = 0, velocity = 0, bow = 0, bowVelocity = 0, ear = 0, earVelocity = 0;
  let rect = host.getBoundingClientRect();
  const clamp = (n, lo, hi) => Math.max(lo, Math.min(hi, n));
  const active = () => !failed && !reduced.matches && !document.hidden;
  const refreshRect = () => { rect = host.getBoundingClientRect(); };
  new ResizeObserver(refreshRect).observe(host);
  window.addEventListener('resize', refreshRect);
  function neutral() {
    for (const name of ['head','tuft','bow','ear-left','ear-right','gaze','eye-left','eye-right']) parts[name].removeAttribute('transform');
    for (const name of ['lid-left','lid-right','blush']) parts[name].setAttribute('opacity','0');
  }
  function sync() {
    cancelAnimationFrame(frame); frame = 0; last = 0;
    document.documentElement.classList.toggle('background-paused', !active());
    if (active()) frame = requestAnimationFrame(tick);
    else neutral();
  }
  reduced.addEventListener('change', sync);
  document.addEventListener('visibilitychange', sync);
  window.addEventListener('pointermove', event => {
    if (!active() || event.pointerType === 'touch') return;
    targetX = clamp((event.clientX - rect.left - rect.width * .4) / (innerWidth * .55), -1, 1);
    targetY = clamp((event.clientY - rect.top - rect.height * .62) / (innerHeight * .55), -1, 1);
  }, { passive: true });
  const relax = () => { targetX = 0; targetY = 0; };
  document.documentElement.addEventListener('pointerleave', relax);
  window.addEventListener('blur', relax);
  let lastPet = -10;
  button.addEventListener('pointermove', event => {
    if (!active() || clock - lastPet < .3) return;
    const py = (event.clientY - rect.top) / rect.height;
    if (py < .45) { velocity += clamp(event.movementX || 0, -15, 15) * 2; lastPet = clock; }
  }, { passive: true });
  button.addEventListener('click', event => {
    if (!active()) return;
    const px = event.detail ? (event.clientX - rect.left) / rect.width : .4;
    const py = event.detail ? (event.clientY - rect.top) / rect.height : .4;
    happyUntil = clock + 1.15; bounce = 1;
    if (py < .3) velocity += 145;
    else if (px > .73) { bowVelocity += 155; earVelocity += 110; }
    else { velocity += 75; bowVelocity += 65; }
    velocity = clamp(velocity,-180,180); bowVelocity = clamp(bowVelocity,-180,180); earVelocity = clamp(earVelocity,-100,100);
  });
  function tick(now) {
    if (!active()) { frame = 0; return; }
    const dt = last ? Math.min((now - last) / 1000, .035) : 1 / 60;
    last = now; clock += dt;
    const ease = 1 - Math.exp(-dt * 7);
    x += (targetX - x) * ease; y += (targetY - y) * ease;
    const tuftTarget = x * 7 + Math.sin(clock * 2.3) * 2;
    velocity += ((tuftTarget - tuft) * 65 - velocity * 9) * dt; tuft += velocity * dt;
    const bowTarget = -x * 5 + Math.sin(clock * 2.7 + 1) * 2.5;
    bowVelocity += ((bowTarget - bow) * 75 - bowVelocity * 10) * dt; bow += bowVelocity * dt;
    const earTarget = -x * 4 + Math.sin(clock * 2.1 + .4) * 1.4;
    earVelocity += ((earTarget - ear) * 58 - earVelocity * 9) * dt; ear += earVelocity * dt;
    bounce *= Math.exp(-dt * 4);
    parts.head.setAttribute('transform', `translate(${x * 8} ${Math.sin(clock * 1.6) * 3 + y * 5 - bounce * 18}) rotate(${x * 2.4} 627 1080)`);
    parts.tuft.setAttribute('transform', `rotate(${clamp(tuft,-9,9)} 686 272)`);
    parts.bow.setAttribute('transform', `rotate(${clamp(bow,-5,5)} 1090 714)`);
    parts['ear-left'].setAttribute('transform', `rotate(${clamp(-ear,-3,3)} 181 575)`);
    parts['ear-right'].setAttribute('transform', `rotate(${clamp(ear,-3,3)} 1088 746)`);
    parts.gaze.setAttribute('transform', `translate(${x * 12} ${y * 9})`);
    if (clock >= nextBlink) { blinkStart = clock; nextBlink = clock + 2.6 + Math.random() * 4; }
    const blinkAge = clock - blinkStart;
    const blink = blinkAge < .19 ? 1 - Math.sin(blinkAge / .19 * Math.PI) * .97 : 1;
    const happy = clock < happyUntil;
    for (const side of ['left','right']) {
      parts[`eye-${side}`].setAttribute('transform', `scale(1 ${happy ? 0 : Math.max(.03,blink)})`);
      parts[`lid-${side}`].setAttribute('opacity', happy ? '1' : '0');
    }
    parts.blush.setAttribute('opacity', happy ? '.42' : '0');
    frame = requestAnimationFrame(tick);
  }
  // If the layer fails to load, keep the original illustration instead of a partial face.
  for (const file of ['head-v2.png', 'accessories-v2.png']) {
    const layer = new Image();
    layer.onerror = () => { failed = true; sync(); host.remove(); document.querySelector('.backdrop-character').style.display = 'block'; };
    layer.src = `/mascot/${file}`;
  }
  sync();
})();
