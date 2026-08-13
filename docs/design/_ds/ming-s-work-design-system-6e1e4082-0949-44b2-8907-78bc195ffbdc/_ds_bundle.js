/* @ds-bundle: {"format":3,"namespace":"MingSWorkDesignSystem_6e1e40","components":[],"sourceHashes":{"ui_kits/mobile/app.jsx":"c16e2c38ba42","ui_kits/mobile/ios-frame.jsx":"be3343be4b51","ui_kits/mobile/screens.jsx":"b3bd8b28227c","ui_kits/mobile/ui.jsx":"dc3d4eb37cbb","ui_kits/portfolio/app.jsx":"01945327b11e","ui_kits/portfolio/sections.jsx":"b31ecea7a4c6","ui_kits/portfolio/ui.jsx":"8c04a814991e","ui_kits/webapp/app.jsx":"a8a1088b74ce","ui_kits/webapp/shell.jsx":"6ca89d4510bd","ui_kits/webapp/ui.jsx":"eb3427d30712","ui_kits/webapp/views.jsx":"c604653ced2a"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.MingSWorkDesignSystem_6e1e40 = window.MingSWorkDesignSystem_6e1e40 || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// ui_kits/mobile/app.jsx
try { (() => {
/* app.jsx — Marginalia (iOS 26) shell + mount */

function GroupedList({
  header,
  children
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      margin: '0 0 22px'
    }
  }, header && /*#__PURE__*/React.createElement("div", {
    className: "t-group-header",
    style: {
      padding: '0 0 7px 16px'
    }
  }, header), /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--card)',
      borderRadius: 18,
      overflow: 'hidden',
      boxShadow: 'var(--shadow-1)'
    }
  }, children));
}
function Row({
  icon,
  color,
  title,
  detail,
  last
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '0 16px',
      minHeight: 48,
      position: 'relative'
    }
  }, /*#__PURE__*/React.createElement(Tile, {
    icon: icon,
    color: color
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      fontFamily: 'var(--font-text)',
      fontSize: 17,
      letterSpacing: '-0.43px',
      color: 'var(--label)'
    }
  }, title), detail && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-text)',
      fontSize: 17,
      color: 'var(--label-secondary)'
    }
  }, detail), /*#__PURE__*/React.createElement(Icon, {
    name: "chevron-right",
    size: 16,
    style: {
      color: 'var(--label-tertiary)'
    }
  }), !last && /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: 58,
      right: 0,
      bottom: 0,
      height: '0.5px',
      background: 'var(--separator)'
    }
  }));
}
function ProfileScreen() {
  return /*#__PURE__*/React.createElement(Screen, null, /*#__PURE__*/React.createElement(LargeTitle, {
    title: "You"
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto',
      padding: '8px 16px 110px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 14,
      padding: '8px 4px 22px'
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: AVATAR,
    alt: "Ming",
    style: {
      width: 64,
      height: 64,
      borderRadius: '50%',
      background: 'var(--fill-tertiary)'
    }
  }), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-display)',
      fontWeight: 700,
      fontSize: 24,
      letterSpacing: '-0.4px',
      color: 'var(--label)'
    }
  }, "Ming"), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-text)',
      fontSize: 14,
      color: 'var(--label-secondary)'
    }
  }, "38 highlights \xB7 4 readings"))), /*#__PURE__*/React.createElement(GroupedList, {
    header: "Reading"
  }, /*#__PURE__*/React.createElement(Row, {
    icon: "book-open",
    color: "var(--blue)",
    title: "Theme",
    detail: "Automatic"
  }), /*#__PURE__*/React.createElement(Row, {
    icon: "type",
    color: "var(--orange)",
    title: "Text Size",
    detail: "Large"
  }), /*#__PURE__*/React.createElement(Row, {
    icon: "cloud",
    color: "var(--green)",
    title: "iCloud Sync",
    detail: "On",
    last: true
  })), /*#__PURE__*/React.createElement(GroupedList, {
    header: "Notes"
  }, /*#__PURE__*/React.createElement(Row, {
    icon: "download",
    color: "var(--indigo)",
    title: "Export",
    detail: "Markdown"
  }), /*#__PURE__*/React.createElement(Row, {
    icon: "bell",
    color: "var(--red)",
    title: "Reminders",
    detail: "Daily",
    last: true
  }))));
}
function SearchScreen({
  onOpen
}) {
  const [q, setQ] = useState('');
  return /*#__PURE__*/React.createElement(LibraryScreen, {
    onOpen: onOpen,
    query: q,
    onSearch: setQ
  });
}
function App() {
  const [tab, setTab] = useState('library');
  const [reading, setReading] = useState(null);
  const [query, setQuery] = useState('');
  useIcons((reading ? reading.id : tab) + query);
  let screen;
  if (reading) screen = /*#__PURE__*/React.createElement(ReaderScreen, {
    d: reading,
    onBack: () => setReading(null)
  });else if (tab === 'library') screen = /*#__PURE__*/React.createElement(LibraryScreen, {
    onOpen: setReading,
    query: query,
    onSearch: setQuery
  });else if (tab === 'highlights') screen = /*#__PURE__*/React.createElement(HighlightsScreen, null);else if (tab === 'search') screen = /*#__PURE__*/React.createElement(SearchScreen, {
    onOpen: setReading
  });else screen = /*#__PURE__*/React.createElement(ProfileScreen, null);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      minHeight: '100vh',
      padding: 24
    }
  }, /*#__PURE__*/React.createElement(IOSDevice, null, /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      position: 'relative'
    }
  }, screen, !reading && /*#__PURE__*/React.createElement(GlassTabBar, {
    active: tab,
    onTab: setTab
  }))));
}
ReactDOM.createRoot(document.getElementById('root')).render(/*#__PURE__*/React.createElement(App, null));
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/mobile/app.jsx", error: String((e && e.message) || e) }); }

// ui_kits/mobile/ios-frame.jsx
try { (() => {
// @ds-adherence-ignore -- omelette starter scaffold (raw elements/hex/px by design)

/* BEGIN USAGE */
// iOS.jsx — Simplified iOS 26 (Liquid Glass) device frame
// Based on the iOS 26 UI Kit + Figma status bar spec. No assets, no deps.
// Exports (to window): IOSDevice, IOSStatusBar, IOSNavBar, IOSGlassPill, IOSList, IOSListRow, IOSKeyboard
//
// Usage — wrap your screen content in <IOSDevice> to get the bezel, status bar
// and home indicator (props: title, dark, keyboard):
//
//   <IOSDevice title="Settings">
//     ...your screen content...
//   </IOSDevice>
//   <IOSDevice dark title="Search" keyboard>…</IOSDevice>
/* END USAGE */

// ─────────────────────────────────────────────────────────────
// Status bar
// ─────────────────────────────────────────────────────────────
function IOSStatusBar({
  dark = false,
  time = '9:41'
}) {
  const c = dark ? '#fff' : '#000';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 154,
      alignItems: 'center',
      justifyContent: 'center',
      padding: '21px 24px 19px',
      boxSizing: 'border-box',
      position: 'relative',
      zIndex: 20,
      width: '100%'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      height: 22,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      paddingTop: 1.5
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: '-apple-system, "SF Pro", system-ui',
      fontWeight: 590,
      fontSize: 17,
      lineHeight: '22px',
      color: c
    }
  }, time)), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      height: 22,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 7,
      paddingTop: 1,
      paddingRight: 1
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "19",
    height: "12",
    viewBox: "0 0 19 12"
  }, /*#__PURE__*/React.createElement("rect", {
    x: "0",
    y: "7.5",
    width: "3.2",
    height: "4.5",
    rx: "0.7",
    fill: c
  }), /*#__PURE__*/React.createElement("rect", {
    x: "4.8",
    y: "5",
    width: "3.2",
    height: "7",
    rx: "0.7",
    fill: c
  }), /*#__PURE__*/React.createElement("rect", {
    x: "9.6",
    y: "2.5",
    width: "3.2",
    height: "9.5",
    rx: "0.7",
    fill: c
  }), /*#__PURE__*/React.createElement("rect", {
    x: "14.4",
    y: "0",
    width: "3.2",
    height: "12",
    rx: "0.7",
    fill: c
  })), /*#__PURE__*/React.createElement("svg", {
    width: "17",
    height: "12",
    viewBox: "0 0 17 12"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M8.5 3.2C10.8 3.2 12.9 4.1 14.4 5.6L15.5 4.5C13.7 2.7 11.2 1.5 8.5 1.5C5.8 1.5 3.3 2.7 1.5 4.5L2.6 5.6C4.1 4.1 6.2 3.2 8.5 3.2Z",
    fill: c
  }), /*#__PURE__*/React.createElement("path", {
    d: "M8.5 6.8C9.9 6.8 11.1 7.3 12 8.2L13.1 7.1C11.8 5.9 10.2 5.1 8.5 5.1C6.8 5.1 5.2 5.9 3.9 7.1L5 8.2C5.9 7.3 7.1 6.8 8.5 6.8Z",
    fill: c
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "8.5",
    cy: "10.5",
    r: "1.5",
    fill: c
  })), /*#__PURE__*/React.createElement("svg", {
    width: "27",
    height: "13",
    viewBox: "0 0 27 13"
  }, /*#__PURE__*/React.createElement("rect", {
    x: "0.5",
    y: "0.5",
    width: "23",
    height: "12",
    rx: "3.5",
    stroke: c,
    strokeOpacity: "0.35",
    fill: "none"
  }), /*#__PURE__*/React.createElement("rect", {
    x: "2",
    y: "2",
    width: "20",
    height: "9",
    rx: "2",
    fill: c
  }), /*#__PURE__*/React.createElement("path", {
    d: "M25 4.5V8.5C25.8 8.2 26.5 7.2 26.5 6.5C26.5 5.8 25.8 4.8 25 4.5Z",
    fill: c,
    fillOpacity: "0.4"
  }))));
}

// ─────────────────────────────────────────────────────────────
// Liquid glass pill — blur + tint + shine
// ─────────────────────────────────────────────────────────────
function IOSGlassPill({
  children,
  dark = false,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: 44,
      minWidth: 44,
      borderRadius: 9999,
      position: 'relative',
      overflow: 'hidden',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      boxShadow: dark ? '0 2px 6px rgba(0,0,0,0.35), 0 6px 16px rgba(0,0,0,0.2)' : '0 1px 3px rgba(0,0,0,0.07), 0 3px 10px rgba(0,0,0,0.06)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      borderRadius: 9999,
      backdropFilter: 'blur(12px) saturate(180%)',
      WebkitBackdropFilter: 'blur(12px) saturate(180%)',
      background: dark ? 'rgba(120,120,128,0.28)' : 'rgba(255,255,255,0.5)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      borderRadius: 9999,
      boxShadow: dark ? 'inset 1.5px 1.5px 1px rgba(255,255,255,0.15), inset -1px -1px 1px rgba(255,255,255,0.08)' : 'inset 1.5px 1.5px 1px rgba(255,255,255,0.7), inset -1px -1px 1px rgba(255,255,255,0.4)',
      border: dark ? '0.5px solid rgba(255,255,255,0.15)' : '0.5px solid rgba(0,0,0,0.06)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      zIndex: 1,
      display: 'flex',
      alignItems: 'center',
      padding: '0 4px'
    }
  }, children));
}

// ─────────────────────────────────────────────────────────────
// Navigation bar — glass pills + large title
// ─────────────────────────────────────────────────────────────
function IOSNavBar({
  title = 'Title',
  dark = false,
  trailingIcon = true
}) {
  const muted = dark ? 'rgba(255,255,255,0.6)' : '#404040';
  const text = dark ? '#fff' : '#000';
  const pillIcon = content => /*#__PURE__*/React.createElement(IOSGlassPill, {
    dark: dark
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: 36,
      height: 36,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, content));
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 10,
      paddingTop: 62,
      paddingBottom: 10,
      position: 'relative',
      zIndex: 5
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '0 16px'
    }
  }, pillIcon(/*#__PURE__*/React.createElement("svg", {
    width: "12",
    height: "20",
    viewBox: "0 0 12 20",
    fill: "none",
    style: {
      marginLeft: -1
    }
  }, /*#__PURE__*/React.createElement("path", {
    d: "M10 2L2 10l8 8",
    stroke: muted,
    strokeWidth: "2.5",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }))), trailingIcon && pillIcon(/*#__PURE__*/React.createElement("svg", {
    width: "22",
    height: "6",
    viewBox: "0 0 22 6"
  }, /*#__PURE__*/React.createElement("circle", {
    cx: "3",
    cy: "3",
    r: "2.5",
    fill: muted
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "11",
    cy: "3",
    r: "2.5",
    fill: muted
  }), /*#__PURE__*/React.createElement("circle", {
    cx: "19",
    cy: "3",
    r: "2.5",
    fill: muted
  })))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '0 16px',
      fontFamily: '-apple-system, system-ui',
      fontSize: 34,
      fontWeight: 700,
      lineHeight: '41px',
      color: text,
      letterSpacing: 0.4
    }
  }, title));
}

// ─────────────────────────────────────────────────────────────
// Grouped list (inset card, r:26) + row (52px)
// ─────────────────────────────────────────────────────────────
function IOSListRow({
  title,
  detail,
  icon,
  chevron = true,
  isLast = false,
  dark = false
}) {
  const text = dark ? '#fff' : '#000';
  const sec = dark ? 'rgba(235,235,245,0.6)' : 'rgba(60,60,67,0.6)';
  const ter = dark ? 'rgba(235,235,245,0.3)' : 'rgba(60,60,67,0.3)';
  const sep = dark ? 'rgba(84,84,88,0.65)' : 'rgba(60,60,67,0.12)';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      minHeight: 52,
      padding: '0 16px',
      position: 'relative',
      fontFamily: '-apple-system, system-ui',
      fontSize: 17,
      letterSpacing: -0.43
    }
  }, icon && /*#__PURE__*/React.createElement("div", {
    style: {
      width: 30,
      height: 30,
      borderRadius: 7,
      background: icon,
      marginRight: 12,
      flexShrink: 0
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      color: text
    }
  }, title), detail && /*#__PURE__*/React.createElement("span", {
    style: {
      color: sec,
      marginRight: 6
    }
  }, detail), chevron && /*#__PURE__*/React.createElement("svg", {
    width: "8",
    height: "14",
    viewBox: "0 0 8 14",
    style: {
      flexShrink: 0
    }
  }, /*#__PURE__*/React.createElement("path", {
    d: "M1 1l6 6-6 6",
    stroke: ter,
    strokeWidth: "2",
    fill: "none",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  })), !isLast && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      bottom: 0,
      right: 0,
      left: icon ? 58 : 16,
      height: 0.5,
      background: sep
    }
  }));
}
function IOSList({
  header,
  children,
  dark = false
}) {
  const hc = dark ? 'rgba(235,235,245,0.6)' : 'rgba(60,60,67,0.6)';
  const bg = dark ? '#1C1C1E' : '#fff';
  return /*#__PURE__*/React.createElement("div", null, header && /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: '-apple-system, system-ui',
      fontSize: 13,
      color: hc,
      textTransform: 'uppercase',
      padding: '8px 36px 6px',
      letterSpacing: -0.08
    }
  }, header), /*#__PURE__*/React.createElement("div", {
    style: {
      background: bg,
      borderRadius: 26,
      margin: '0 16px',
      overflow: 'hidden'
    }
  }, children));
}

// ─────────────────────────────────────────────────────────────
// Device frame
// ─────────────────────────────────────────────────────────────
function IOSDevice({
  children,
  width = 402,
  height = 874,
  dark = false,
  title,
  keyboard = false
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      width,
      height,
      borderRadius: 48,
      overflow: 'hidden',
      position: 'relative',
      background: dark ? '#000' : '#F2F2F7',
      boxShadow: '0 40px 80px rgba(0,0,0,0.18), 0 0 0 1px rgba(0,0,0,0.12)',
      fontFamily: '-apple-system, system-ui, sans-serif',
      WebkitFontSmoothing: 'antialiased'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 11,
      left: '50%',
      transform: 'translateX(-50%)',
      width: 126,
      height: 37,
      borderRadius: 24,
      background: '#000',
      zIndex: 50
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 0,
      left: 0,
      right: 0,
      zIndex: 10
    }
  }, /*#__PURE__*/React.createElement(IOSStatusBar, {
    dark: dark
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      display: 'flex',
      flexDirection: 'column'
    }
  }, title !== undefined && /*#__PURE__*/React.createElement(IOSNavBar, {
    title: title,
    dark: dark
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto'
    }
  }, children), keyboard && /*#__PURE__*/React.createElement(IOSKeyboard, {
    dark: dark
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      bottom: 0,
      left: 0,
      right: 0,
      zIndex: 60,
      height: 34,
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'flex-end',
      paddingBottom: 8,
      pointerEvents: 'none'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: 139,
      height: 5,
      borderRadius: 100,
      background: dark ? 'rgba(255,255,255,0.7)' : 'rgba(0,0,0,0.25)'
    }
  })));
}

// ─────────────────────────────────────────────────────────────
// Keyboard — iOS 26 liquid glass
// ─────────────────────────────────────────────────────────────
function IOSKeyboard({
  dark = false
}) {
  const glyph = dark ? 'rgba(255,255,255,0.7)' : '#595959';
  const sugg = dark ? 'rgba(255,255,255,0.6)' : '#333';
  const keyBg = dark ? 'rgba(255,255,255,0.22)' : 'rgba(255,255,255,0.85)';

  // special-key icons
  const icons = {
    shift: /*#__PURE__*/React.createElement("svg", {
      width: "19",
      height: "17",
      viewBox: "0 0 19 17"
    }, /*#__PURE__*/React.createElement("path", {
      d: "M9.5 1L1 9.5h4.5V16h8V9.5H18L9.5 1z",
      fill: glyph
    })),
    del: /*#__PURE__*/React.createElement("svg", {
      width: "23",
      height: "17",
      viewBox: "0 0 23 17"
    }, /*#__PURE__*/React.createElement("path", {
      d: "M7 1h13a2 2 0 012 2v11a2 2 0 01-2 2H7l-6-7.5L7 1z",
      fill: "none",
      stroke: glyph,
      strokeWidth: "1.6",
      strokeLinejoin: "round"
    }), /*#__PURE__*/React.createElement("path", {
      d: "M10 5l7 7M17 5l-7 7",
      stroke: glyph,
      strokeWidth: "1.6",
      strokeLinecap: "round"
    })),
    ret: /*#__PURE__*/React.createElement("svg", {
      width: "20",
      height: "14",
      viewBox: "0 0 20 14"
    }, /*#__PURE__*/React.createElement("path", {
      d: "M18 1v6H4m0 0l4-4M4 7l4 4",
      fill: "none",
      stroke: "#fff",
      strokeWidth: "1.8",
      strokeLinecap: "round",
      strokeLinejoin: "round"
    }))
  };
  const key = (content, {
    w,
    flex,
    ret,
    fs = 25,
    k
  } = {}) => /*#__PURE__*/React.createElement("div", {
    key: k,
    style: {
      height: 42,
      borderRadius: 8.5,
      flex: flex ? 1 : undefined,
      width: w,
      minWidth: 0,
      background: ret ? '#08f' : keyBg,
      boxShadow: '0 1px 0 rgba(0,0,0,0.075)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: '-apple-system, "SF Compact", system-ui',
      fontSize: fs,
      fontWeight: 458,
      color: ret ? '#fff' : glyph
    }
  }, content);
  const row = (keys, pad = 0) => /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6.5,
      justifyContent: 'center',
      padding: `0 ${pad}px`
    }
  }, keys.map(l => key(l, {
    flex: true,
    k: l
  })));
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      zIndex: 15,
      borderRadius: 27,
      overflow: 'hidden',
      padding: '11px 0 2px',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      boxShadow: dark ? '0 -2px 20px rgba(0,0,0,0.09)' : '0 -1px 6px rgba(0,0,0,0.018), 0 -3px 20px rgba(0,0,0,0.012)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      borderRadius: 27,
      backdropFilter: 'blur(12px) saturate(180%)',
      WebkitBackdropFilter: 'blur(12px) saturate(180%)',
      background: dark ? 'rgba(120,120,128,0.14)' : 'rgba(255,255,255,0.25)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      borderRadius: 27,
      boxShadow: dark ? 'inset 1.5px 1.5px 1px rgba(255,255,255,0.15)' : 'inset 1.5px 1.5px 1px rgba(255,255,255,0.7), inset -1px -1px 1px rgba(255,255,255,0.4)',
      border: dark ? '0.5px solid rgba(255,255,255,0.15)' : '0.5px solid rgba(0,0,0,0.06)',
      pointerEvents: 'none'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 20,
      alignItems: 'center',
      padding: '8px 22px 13px',
      width: '100%',
      boxSizing: 'border-box',
      position: 'relative'
    }
  }, ['"The"', 'the', 'to'].map((w, i) => /*#__PURE__*/React.createElement(React.Fragment, {
    key: i
  }, i > 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      width: 1,
      height: 25,
      background: '#ccc',
      opacity: 0.3
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      textAlign: 'center',
      fontFamily: '-apple-system, system-ui',
      fontSize: 17,
      color: sugg,
      letterSpacing: -0.43,
      lineHeight: '22px'
    }
  }, w)))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 13,
      padding: '0 6.5px',
      width: '100%',
      boxSizing: 'border-box',
      position: 'relative'
    }
  }, row(['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p']), row(['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'], 20), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 14.25,
      alignItems: 'center'
    }
  }, key(icons.shift, {
    w: 45,
    k: 'shift'
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6.5,
      flex: 1
    }
  }, ['z', 'x', 'c', 'v', 'b', 'n', 'm'].map(l => key(l, {
    flex: true,
    k: l
  }))), key(icons.del, {
    w: 45,
    k: 'del'
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6,
      alignItems: 'center'
    }
  }, key('ABC', {
    w: 92.25,
    fs: 18,
    k: 'abc'
  }), key('', {
    flex: true,
    k: 'space'
  }), key(icons.ret, {
    w: 92.25,
    ret: true,
    k: 'ret'
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      height: 56,
      width: '100%',
      position: 'relative'
    }
  }));
}
Object.assign(window, {
  IOSDevice,
  IOSStatusBar,
  IOSNavBar,
  IOSGlassPill,
  IOSList,
  IOSListRow,
  IOSKeyboard
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/mobile/ios-frame.jsx", error: String((e && e.message) || e) }); }

// ui_kits/mobile/screens.jsx
try { (() => {
/* screens.jsx — Marginalia (iOS 26) screens */

function Screen({
  children
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      display: 'flex',
      flexDirection: 'column',
      background: 'var(--bg-grouped)'
    }
  }, children);
}
function LargeTitle({
  title,
  trailing,
  search,
  onSearch,
  query
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '58px 20px 8px',
      flex: '0 0 auto',
      background: 'var(--bg-grouped)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      minHeight: 44
    }
  }, /*#__PURE__*/React.createElement("h1", {
    className: "t-large-title",
    style: {
      margin: 0,
      fontSize: 34,
      fontWeight: 700,
      letterSpacing: '0.37px'
    }
  }, title), trailing), search && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      background: 'var(--fill-tertiary)',
      borderRadius: 10,
      padding: '9px 11px',
      marginTop: 12
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "search",
    size: 17,
    style: {
      color: 'var(--label-secondary)'
    }
  }), /*#__PURE__*/React.createElement("input", {
    value: query,
    onChange: e => onSearch(e.target.value),
    placeholder: "Search",
    style: {
      border: 'none',
      outline: 'none',
      background: 'transparent',
      flex: 1,
      fontFamily: 'var(--font-text)',
      fontSize: 17,
      letterSpacing: '-0.4px',
      color: 'var(--label)'
    }
  })));
}
function AvatarBtn({
  onClick
}) {
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    style: {
      padding: 0,
      border: 'none',
      background: 'none',
      cursor: 'pointer'
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: AVATAR,
    alt: "You",
    style: {
      width: 34,
      height: 34,
      borderRadius: '50%',
      background: 'var(--fill-tertiary)',
      display: 'block'
    }
  }));
}
function ReadingCell({
  d,
  onOpen
}) {
  const [p, setP] = useState(false);
  return /*#__PURE__*/React.createElement("button", {
    onClick: () => onOpen(d),
    onPointerDown: () => setP(true),
    onPointerUp: () => setP(false),
    onPointerLeave: () => setP(false),
    style: {
      display: 'flex',
      gap: 14,
      width: '100%',
      textAlign: 'left',
      cursor: 'pointer',
      alignItems: 'center',
      background: 'var(--card)',
      border: 'none',
      borderRadius: 18,
      padding: 12,
      boxShadow: 'var(--shadow-1)',
      transform: p ? 'scale(0.98)' : 'none',
      transition: 'transform .15s'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 52,
      height: 66,
      borderRadius: 9,
      flex: '0 0 auto',
      background: d.color,
      position: 'relative',
      boxShadow: '0 2px 6px rgba(0,0,0,.15)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      inset: 0,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-serif)',
      fontSize: 26,
      color: 'rgba(255,255,255,.95)'
    }
  }, d.title[0])), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontFamily: 'var(--font-text)',
      fontWeight: 600,
      fontSize: 17,
      letterSpacing: '-0.43px',
      color: 'var(--label)',
      lineHeight: 1.2,
      marginBottom: 3
    }
  }, d.title), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      fontFamily: 'var(--font-text)',
      fontSize: 14,
      color: 'var(--label-secondary)',
      marginBottom: 9
    }
  }, d.author, " \xB7 ", d.minutes, " min"), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      height: 4,
      borderRadius: 2,
      background: 'var(--fill)',
      overflow: 'hidden',
      display: 'block'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      width: d.progress + '%',
      height: '100%',
      background: d.color,
      borderRadius: 2
    }
  })), d.notes > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 3,
      fontFamily: 'var(--font-text)',
      fontSize: 12,
      color: 'var(--label-secondary)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "highlighter",
    size: 12
  }), d.notes))));
}
function LibraryScreen({
  onOpen,
  query,
  onSearch
}) {
  const list = READINGS.filter(d => (d.title + d.author).toLowerCase().includes(query.toLowerCase()));
  return /*#__PURE__*/React.createElement(Screen, null, /*#__PURE__*/React.createElement(LargeTitle, {
    title: "Library",
    search: true,
    query: query,
    onSearch: onSearch,
    trailing: /*#__PURE__*/React.createElement(AvatarBtn, null)
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto',
      padding: '12px 16px 110px',
      display: 'flex',
      flexDirection: 'column',
      gap: 12
    }
  }, list.map(d => /*#__PURE__*/React.createElement(ReadingCell, {
    key: d.id,
    d: d,
    onOpen: onOpen
  })), list.length === 0 && /*#__PURE__*/React.createElement("p", {
    style: {
      textAlign: 'center',
      color: 'var(--label-tertiary)',
      fontFamily: 'var(--font-text)',
      marginTop: 40
    }
  }, "No results")));
}
function HighlightCard({
  h
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--card)',
      borderRadius: 18,
      padding: '16px 16px 14px',
      boxShadow: 'var(--shadow-1)',
      position: 'relative',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: 0,
      top: 14,
      bottom: 14,
      width: 4,
      borderRadius: 2,
      background: h.color
    }
  }), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-serif)',
      fontSize: 18,
      lineHeight: 1.5,
      color: 'var(--label)',
      margin: '0 0 12px',
      paddingLeft: 12
    }
  }, h.text), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      paddingLeft: 12
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "book-open",
    size: 13,
    style: {
      color: 'var(--label-tertiary)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-text)',
      fontSize: 13,
      color: 'var(--label-secondary)'
    }
  }, h.src)), h.note && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 12,
      marginLeft: 12,
      padding: '10px 12px',
      background: 'var(--fill-quaternary)',
      borderRadius: 12,
      display: 'flex',
      gap: 8,
      alignItems: 'flex-start'
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: AVATAR,
    alt: "",
    style: {
      width: 18,
      height: 18,
      borderRadius: '50%',
      flex: '0 0 auto'
    }
  }), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-text)',
      fontSize: 14,
      lineHeight: 1.4,
      color: 'var(--label-secondary)',
      margin: 0
    }
  }, h.note)));
}
function HighlightsScreen() {
  return /*#__PURE__*/React.createElement(Screen, null, /*#__PURE__*/React.createElement(LargeTitle, {
    title: "Highlights",
    trailing: /*#__PURE__*/React.createElement(AvatarBtn, null)
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '0 20px 8px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    className: "t-footnote"
  }, HIGHLIGHTS.length, " saved \xB7 synced")), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto',
      padding: '8px 16px 110px',
      display: 'flex',
      flexDirection: 'column',
      gap: 12
    }
  }, HIGHLIGHTS.map(h => /*#__PURE__*/React.createElement(HighlightCard, {
    key: h.id,
    h: h
  }))));
}
function Para({
  block
}) {
  const st = {
    fontFamily: 'var(--font-serif)',
    fontSize: 19,
    lineHeight: 1.65,
    color: 'var(--label)',
    margin: 0
  };
  if (typeof block === 'string') return /*#__PURE__*/React.createElement("p", {
    style: st
  }, block);
  const idx = block.t.indexOf(block.hi);
  const parts = idx >= 0 ? [block.t.slice(0, idx), /*#__PURE__*/React.createElement("mark", {
    key: "m",
    style: {
      background: 'color-mix(in srgb, var(--tint) 22%, transparent)',
      color: 'inherit',
      borderRadius: 3,
      padding: '0 1px'
    }
  }, block.hi), block.t.slice(idx + block.hi.length)] : block.t;
  return /*#__PURE__*/React.createElement("p", {
    style: st
  }, parts);
}
function ReaderScreen({
  d,
  onBack
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      display: 'flex',
      flexDirection: 'column',
      background: 'var(--bg)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '54px 14px 10px',
      flex: '0 0 auto',
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      position: 'sticky',
      top: 0,
      zIndex: 5,
      background: 'var(--glass-thick)',
      WebkitBackdropFilter: 'var(--glass-blur)',
      backdropFilter: 'var(--glass-blur)',
      borderBottom: '0.5px solid var(--separator)'
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: onBack,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 2,
      background: 'none',
      border: 'none',
      cursor: 'pointer',
      color: 'var(--tint)',
      fontFamily: 'var(--font-text)',
      fontSize: 17,
      letterSpacing: '-0.4px',
      padding: 0
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "chevron-left",
    size: 22,
    stroke: 2.2
  }), "Library"), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      textAlign: 'center',
      fontFamily: 'var(--font-text)',
      fontWeight: 600,
      fontSize: 16,
      color: 'var(--label)',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      maxWidth: 180,
      margin: '0 auto'
    }
  }, d.title), /*#__PURE__*/React.createElement("button", {
    style: {
      background: 'none',
      border: 'none',
      cursor: 'pointer',
      color: 'var(--tint)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "bookmark",
    size: 20
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto',
      padding: '24px 24px 60px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    className: "t-footnote",
    style: {
      color: 'var(--tint)',
      fontWeight: 600
    }
  }, d.author.toUpperCase()), /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: 'var(--font-serif)',
      fontSize: 30,
      fontWeight: 700,
      letterSpacing: '-0.3px',
      color: 'var(--label)',
      margin: '8px 0 22px',
      lineHeight: 1.12
    }
  }, d.title), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 18
    }
  }, d.body.map((b, i) => /*#__PURE__*/React.createElement(Para, {
    key: i,
    block: b
  })))));
}
const TABS = [{
  id: 'library',
  icon: 'book-open',
  label: 'Read'
}, {
  id: 'highlights',
  icon: 'highlighter',
  label: 'Notes'
}, {
  id: 'search',
  icon: 'search',
  label: 'Search'
}, {
  id: 'profile',
  icon: 'user',
  label: 'You'
}];
function GlassTabBar({
  active,
  onTab
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      bottom: 16,
      left: '50%',
      transform: 'translateX(-50%)',
      zIndex: 40,
      display: 'flex',
      gap: 4,
      padding: '7px 9px',
      borderRadius: 9999,
      background: 'var(--glass-thick)',
      WebkitBackdropFilter: 'var(--glass-blur-lg)',
      backdropFilter: 'var(--glass-blur-lg)',
      boxShadow: 'var(--glass-shine), var(--glass-shadow)',
      border: 'var(--glass-hairline)'
    }
  }, TABS.map(t => {
    const on = active === t.id;
    return /*#__PURE__*/React.createElement("button", {
      key: t.id,
      onClick: () => onTab(t.id),
      style: {
        background: on ? 'color-mix(in srgb, var(--tint) 16%, transparent)' : 'transparent',
        border: 'none',
        cursor: 'pointer',
        width: 62,
        height: 48,
        borderRadius: 9999,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 3,
        color: on ? 'var(--tint)' : 'var(--label-secondary)'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: t.icon,
      size: 22,
      stroke: on ? 2.1 : 1.7
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-text)',
        fontSize: 10,
        fontWeight: on ? 600 : 500
      }
    }, t.label));
  }));
}
Object.assign(window, {
  Screen,
  LargeTitle,
  AvatarBtn,
  ReadingCell,
  LibraryScreen,
  HighlightCard,
  HighlightsScreen,
  Para,
  ReaderScreen,
  GlassTabBar
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/mobile/screens.jsx", error: String((e && e.message) || e) }); }

// ui_kits/mobile/ui.jsx
try { (() => {
/* ui.jsx — primitives + data for Marginalia (iOS 26) */
const {
  useState,
  useEffect,
  useLayoutEffect
} = React;
function useIcons(dep) {
  useLayoutEffect(() => {
    if (window.lucide) window.lucide.createIcons();
  }, [dep]);
}
function Icon({
  name,
  size = 18,
  stroke = 1.7,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("i", {
    "data-lucide": name,
    style: {
      width: size,
      height: size,
      strokeWidth: stroke,
      display: 'inline-flex',
      ...style
    }
  });
}
const AVATAR = '../../assets/avatar.png';

/* iOS capsule button */
function Button({
  children,
  kind = 'filled',
  onClick,
  style = {},
  icon
}) {
  const [p, setP] = useState(false);
  const kinds = {
    filled: {
      background: p ? 'var(--tint-deep)' : 'var(--tint)',
      color: '#fff'
    },
    tinted: {
      background: 'color-mix(in srgb, var(--tint) 15%, transparent)',
      color: 'var(--tint)'
    },
    gray: {
      background: 'var(--fill-tertiary)',
      color: 'var(--label)'
    },
    plain: {
      background: 'transparent',
      color: 'var(--tint)'
    }
  };
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    onPointerDown: () => setP(true),
    onPointerUp: () => setP(false),
    onPointerLeave: () => setP(false),
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 6,
      border: 'none',
      cursor: 'pointer',
      fontFamily: 'var(--font-text)',
      fontWeight: 600,
      fontSize: 17,
      letterSpacing: '-0.4px',
      padding: '11px 20px',
      borderRadius: 9999,
      transition: 'background .15s',
      ...kinds[kind],
      ...style
    }
  }, icon && /*#__PURE__*/React.createElement(Icon, {
    name: icon,
    size: 17
  }), children);
}

/* leading SF-symbol-style tile for list rows */
function Tile({
  icon,
  color
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      width: 30,
      height: 30,
      borderRadius: 7,
      background: color,
      flex: '0 0 auto',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: '#fff'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: icon,
    size: 18,
    stroke: 2
  }));
}
const READINGS = [{
  id: 'attention',
  title: 'On Paying Attention',
  author: 'Ada Reyes',
  minutes: 12,
  progress: 64,
  notes: 3,
  color: 'var(--blue)',
  body: ['Attention is the rarest and purest form of generosity. We talk about it like a resource to be spent, but it behaves more like a muscle.', {
    t: 'A margin used to be a private place, the one part of a book that belonged entirely to you.',
    hi: 'A margin used to be a private place'
  }, 'What we annotate, we remember. What we merely highlight, we forget twice.']
}, {
  id: 'small-tools',
  title: 'In Praise of Small Tools',
  author: 'Ming',
  minutes: 6,
  progress: 100,
  notes: 2,
  color: 'var(--indigo)',
  body: ['A small tool fits in your head. You can hold all of it at once — every screen, every state, every decision.', {
    t: 'Small software just works, because there is nothing to onboard into.',
    hi: 'Small software just works'
  }]
}, {
  id: 'plain-text',
  title: 'The Case for Plain Text',
  author: 'J. Okonkwo',
  minutes: 9,
  progress: 22,
  notes: 1,
  color: 'var(--orange)',
  body: [{
    t: 'Plain text outlives its tools. Formats are promises, and plain text keeps them.',
    hi: 'Formats are promises'
  }, 'Own your file, and you own your future. Everything else is a lease.']
}, {
  id: 'walking',
  title: 'Walking as a Way of Knowing',
  author: 'Lena Hart',
  minutes: 15,
  progress: 0,
  notes: 0,
  color: 'var(--green)',
  body: ['To walk is to think with the whole body. The pace of the feet sets the pace of the mind.']
}];
const HIGHLIGHTS = [{
  id: 1,
  text: 'A margin used to be a private place, the one part of a book that belonged entirely to you.',
  src: 'On Paying Attention',
  color: 'var(--blue)',
  note: 'The whole thesis, really.'
}, {
  id: 2,
  text: 'Small software just works, because there is nothing to onboard into.',
  src: 'In Praise of Small Tools',
  color: 'var(--indigo)',
  note: 'No onboarding is the best onboarding.'
}, {
  id: 3,
  text: 'Formats are promises, and plain text keeps them.',
  src: 'The Case for Plain Text',
  color: 'var(--orange)'
}];
Object.assign(window, {
  useState,
  useEffect,
  useLayoutEffect,
  useIcons,
  Icon,
  AVATAR,
  Button,
  Tile,
  READINGS,
  HIGHLIGHTS
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/mobile/ui.jsx", error: String((e && e.message) || e) }); }

// ui_kits/portfolio/app.jsx
try { (() => {
/* app.jsx — ProjectDetail + App shell + mount */

function ProjectDetail({
  p,
  onBack
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 'var(--maxw)',
      margin: '0 auto',
      padding: '40px 40px 24px'
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: onBack,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 8,
      background: 'none',
      border: 'none',
      cursor: 'pointer',
      fontFamily: 'var(--font-sans)',
      fontSize: 14.5,
      fontWeight: 500,
      color: 'var(--ink-500)',
      padding: 0,
      marginBottom: 32,
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "arrow-left",
    size: 16
  }), " All work"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-end',
      justifyContent: 'space-between',
      gap: 24,
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(Label, {
    style: {
      color: 'var(--terracotta-deep)'
    }
  }, p.n, " \xB7 ", p.year), /*#__PURE__*/React.createElement("h1", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'clamp(40px,5.5vw,68px)',
      letterSpacing: '-0.025em',
      color: 'var(--ink-900)',
      margin: '12px 0 10px',
      lineHeight: 1
    }
  }, p.name), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 20,
      color: 'var(--ink-500)',
      margin: 0
    }
  }, p.tagline)), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 10,
      alignItems: 'center'
    }
  }, /*#__PURE__*/React.createElement(StatusChip, {
    status: p.status
  }), /*#__PURE__*/React.createElement(Btn, {
    variant: "primary",
    icon: "arrow-up-right"
  }, "Visit"))), /*#__PURE__*/React.createElement("div", {
    style: {
      height: 320,
      background: p.grad,
      borderRadius: 'var(--r-xl)',
      margin: '36px 0 44px',
      boxShadow: 'var(--shadow-md)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 320px',
      gap: 64,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 19,
      lineHeight: 1.7,
      color: 'var(--ink-700)',
      margin: '0 0 28px',
      textWrap: 'pretty'
    }
  }, p.summary), /*#__PURE__*/React.createElement(Label, null, "What made it good"), /*#__PURE__*/React.createElement("ul", {
    style: {
      listStyle: 'none',
      padding: 0,
      margin: '16px 0 0',
      display: 'flex',
      flexDirection: 'column',
      gap: 14
    }
  }, p.points.map((pt, i) => /*#__PURE__*/React.createElement("li", {
    key: i,
    style: {
      display: 'flex',
      gap: 12,
      fontFamily: 'var(--font-sans)',
      fontSize: 16,
      lineHeight: 1.55,
      color: 'var(--ink-700)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--terracotta)',
      marginTop: 2,
      flex: '0 0 auto'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "check",
    size: 17
  })), pt)))), /*#__PURE__*/React.createElement("aside", {
    style: {
      background: 'var(--card)',
      border: '1px solid var(--line)',
      borderRadius: 'var(--r-lg)',
      padding: '22px 24px',
      boxShadow: 'var(--shadow-sm)',
      position: 'sticky',
      top: 88
    }
  }, /*#__PURE__*/React.createElement(Label, null, "Details"), /*#__PURE__*/React.createElement("dl", {
    style: {
      margin: '16px 0 0'
    }
  }, [['Year', p.year], ['Role', p.role], ['Status', STATUS[p.status].label]].map(([k, v]) => /*#__PURE__*/React.createElement("div", {
    key: k,
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      padding: '10px 0',
      borderBottom: '1px solid var(--line)'
    }
  }, /*#__PURE__*/React.createElement("dt", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 11,
      letterSpacing: '.06em',
      textTransform: 'uppercase',
      color: 'var(--ink-400)'
    }
  }, k), /*#__PURE__*/React.createElement("dd", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 14,
      color: 'var(--ink-900)',
      margin: 0,
      textAlign: 'right',
      maxWidth: '60%'
    }
  }, v)))), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 18
    }
  }, /*#__PURE__*/React.createElement(Label, null, "Built with"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexWrap: 'wrap',
      gap: 7,
      marginTop: 12
    }
  }, p.tech.map(t => /*#__PURE__*/React.createElement(Tag, {
    key: t
  }, t)))))));
}
function App() {
  const [active, setActive] = useState(null);
  useIcons(active ? active.id : 'home');
  useEffect(() => {
    window.scrollTo({
      top: 0
    });
  }, [active]);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      minHeight: '100vh',
      background: 'var(--paper)'
    }
  }, /*#__PURE__*/React.createElement(Header, {
    onHome: () => setActive(null)
  }), active ? /*#__PURE__*/React.createElement(ProjectDetail, {
    p: active,
    onBack: () => setActive(null)
  }) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(Hero, null), /*#__PURE__*/React.createElement(WorkList, {
    onOpen: setActive
  }), /*#__PURE__*/React.createElement(About, null)), /*#__PURE__*/React.createElement(Footer, null));
}
ReactDOM.createRoot(document.getElementById('root')).render(/*#__PURE__*/React.createElement(App, null));
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/portfolio/app.jsx", error: String((e && e.message) || e) }); }

// ui_kits/portfolio/sections.jsx
try { (() => {
/* sections.jsx — page sections for the portfolio kit */

function Header({
  onHome
}) {
  const links = ['Work', 'Notes', 'About'];
  return /*#__PURE__*/React.createElement("header", {
    style: {
      position: 'sticky',
      top: 0,
      zIndex: 20,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '18px 40px',
      background: 'rgba(247,242,232,0.82)',
      backdropFilter: 'saturate(1.4) blur(10px)',
      borderBottom: '1px solid var(--line)'
    }
  }, /*#__PURE__*/React.createElement(Logo, {
    size: 28,
    onClick: onHome
  }), /*#__PURE__*/React.createElement("nav", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 28
    }
  }, links.map(l => /*#__PURE__*/React.createElement("a", {
    key: l,
    href: "#",
    onClick: e => e.preventDefault(),
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 15,
      fontWeight: 500,
      color: 'var(--ink-700)',
      textDecoration: 'none'
    }
  }, l)), /*#__PURE__*/React.createElement(Btn, {
    variant: "dark",
    size: "sm",
    icon: "arrow-up-right"
  }, "Get in touch")));
}
function Hero() {
  return /*#__PURE__*/React.createElement("section", {
    style: {
      maxWidth: 'var(--maxw)',
      margin: '0 auto',
      padding: '96px 40px 64px'
    }
  }, /*#__PURE__*/React.createElement(Label, {
    style: {
      color: 'var(--terracotta-deep)'
    }
  }, "Independent software \xB7 since 2014"), /*#__PURE__*/React.createElement("h1", {
    style: {
      fontFamily: 'var(--font-display)',
      fontWeight: 400,
      fontSize: 'clamp(44px,6.5vw,82px)',
      lineHeight: 1.0,
      letterSpacing: '-0.025em',
      color: 'var(--ink-900)',
      margin: '20px 0 0',
      maxWidth: 16 + 'ch',
      textWrap: 'balance'
    }
  }, "Practical software, ", /*#__PURE__*/React.createElement("span", {
    style: {
      fontStyle: 'italic',
      color: 'var(--terracotta-deep)'
    }
  }, "finished"), " to the last detail."), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 20,
      lineHeight: 1.6,
      color: 'var(--ink-500)',
      margin: '28px 0 0',
      maxWidth: '46ch',
      textWrap: 'pretty'
    }
  }, "I'm Ming. I design and build small tools I want to exist \u2014 mostly mine, mostly finished. A few of them are below."), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 12,
      marginTop: 36
    }
  }, /*#__PURE__*/React.createElement(Btn, {
    variant: "primary",
    icon: "arrow-down"
  }, "See the work"), /*#__PURE__*/React.createElement(Btn, {
    variant: "secondary",
    icon: "github"
  }, "GitHub")));
}
function ProjectCard({
  p,
  onOpen
}) {
  const [hover, setHover] = useState(false);
  return /*#__PURE__*/React.createElement("article", {
    onClick: () => onOpen(p),
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      background: 'var(--card)',
      border: '1px solid var(--line)',
      borderRadius: 'var(--r-lg)',
      boxShadow: hover ? 'var(--shadow-md)' : 'var(--shadow-sm)',
      overflow: 'hidden',
      cursor: 'pointer',
      transform: hover ? 'translateY(-3px)' : 'none',
      transition: 'all var(--dur) var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      height: 150,
      background: p.grad,
      position: 'relative'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: 16,
      left: 18,
      fontFamily: 'var(--font-mono)',
      fontSize: 12,
      color: 'rgba(255,255,255,.85)',
      letterSpacing: '.08em'
    }
  }, p.n), /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: 14,
      right: 16,
      width: 30,
      height: 30,
      borderRadius: '50%',
      background: 'rgba(255,255,255,.2)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: '#fff',
      transform: hover ? 'translate(2px,-2px)' : 'none',
      transition: 'transform var(--dur) var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "arrow-up-right",
    size: 17
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '18px 20px 20px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement("h3", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 24,
      letterSpacing: '-0.01em',
      color: 'var(--ink-900)',
      margin: 0
    }
  }, p.name), /*#__PURE__*/React.createElement(StatusChip, {
    status: p.status
  })), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 14.5,
      lineHeight: 1.5,
      color: 'var(--ink-500)',
      margin: '0 0 14px'
    }
  }, p.tagline), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 7
    }
  }, p.tech.map(t => /*#__PURE__*/React.createElement(Tag, {
    key: t
  }, t)))));
}
function WorkList({
  onOpen
}) {
  return /*#__PURE__*/React.createElement("section", {
    id: "work",
    style: {
      maxWidth: 'var(--maxw)',
      margin: '0 auto',
      padding: '48px 40px 40px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      justifyContent: 'space-between',
      borderBottom: '1px solid var(--line)',
      paddingBottom: 16,
      marginBottom: 32
    }
  }, /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'clamp(30px,4vw,44px)',
      letterSpacing: '-0.02em',
      color: 'var(--ink-900)',
      margin: 0
    }
  }, "Selected work"), /*#__PURE__*/React.createElement(Label, null, PROJECTS.length, " projects \xB7 2022 \u2014 now")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: 'repeat(2, 1fr)',
      gap: 24
    }
  }, PROJECTS.map(p => /*#__PURE__*/React.createElement(ProjectCard, {
    key: p.id,
    p: p,
    onOpen: onOpen
  }))));
}
function About() {
  return /*#__PURE__*/React.createElement("section", {
    style: {
      maxWidth: 'var(--maxw)',
      margin: '0 auto',
      padding: '56px 40px 24px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1.4fr',
      gap: 64,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(Label, null, "About"), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-display)',
      fontStyle: 'italic',
      fontSize: 28,
      lineHeight: 1.35,
      color: 'var(--ink-900)',
      margin: '16px 0 0',
      textWrap: 'balance'
    }
  }, "\"I've been making software for a decade \u2014 mostly small, mostly mine.\"")), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 17,
      lineHeight: 1.7,
      color: 'var(--ink-700)',
      margin: 0,
      maxWidth: '52ch',
      textWrap: 'pretty'
    }
  }, "I work alone, end to end \u2014 design, build, ship, support. I like problems where the right answer is to remove something rather than add it. The tools here are the ones I reach for in my own day; if they're useful to you too, all the better."), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 12,
      marginTop: 28
    }
  }, /*#__PURE__*/React.createElement(Btn, {
    variant: "secondary",
    icon: "mail"
  }, "Email me"), /*#__PURE__*/React.createElement(Btn, {
    variant: "ghost",
    icon: "download"
  }, "R\xE9sum\xE9 (PDF)")))));
}
function Footer() {
  return /*#__PURE__*/React.createElement("footer", {
    style: {
      borderTop: '1px solid var(--line)',
      marginTop: 64
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 'var(--maxw)',
      margin: '0 auto',
      padding: '32px 40px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      flexWrap: 'wrap',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement(Logo, {
    size: 24
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 22,
      alignItems: 'center'
    }
  }, ['github', 'mail', 'bookmark'].map(i => /*#__PURE__*/React.createElement("a", {
    key: i,
    href: "#",
    onClick: e => e.preventDefault(),
    style: {
      color: 'var(--ink-500)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: i,
    size: 18
  }))), /*#__PURE__*/React.createElement(Label, {
    style: {
      color: 'var(--ink-300)'
    }
  }, "\xA9 2025 ming's work"))));
}
Object.assign(window, {
  Header,
  Hero,
  WorkList,
  ProjectCard,
  About,
  Footer
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/portfolio/sections.jsx", error: String((e && e.message) || e) }); }

// ui_kits/portfolio/ui.jsx
try { (() => {
/* ui.jsx — shared primitives + data for the ming's work portfolio kit
   Exposes everything on window for the other babel scripts. */
const {
  useState,
  useEffect,
  useRef,
  useLayoutEffect
} = React;

/* Re-render Lucide icons after React commits. */
function useIcons(dep) {
  useLayoutEffect(() => {
    if (window.lucide) window.lucide.createIcons();
  }, [dep]);
}

/* Inline icon — renders a placeholder Lucide swaps to <svg>. */
function Icon({
  name,
  size = 16,
  stroke = 1.75,
  className = '',
  style = {}
}) {
  return /*#__PURE__*/React.createElement("i", {
    "data-lucide": name,
    style: {
      width: size,
      height: size,
      strokeWidth: stroke,
      display: 'inline-flex',
      ...style
    },
    className: className
  });
}
function Logo({
  size = 30,
  onClick
}) {
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClick,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 11,
      cursor: onClick ? 'pointer' : 'default'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: size,
      height: size,
      borderRadius: size * 0.23,
      background: 'var(--terracotta)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      flex: '0 0 auto'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: size * 0.6,
    height: size * 0.6,
    viewBox: "0 0 48 48",
    fill: "none"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M13 34 V21.5 Q13 15.5 18.75 15.5 Q24.5 15.5 24.5 21.5 V34 M24.5 21.5 Q24.5 15.5 30.25 15.5 Q36 15.5 36 21.5 V34",
    stroke: "#FCFAF3",
    strokeWidth: "3.4",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  }))), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: size * 0.62,
      letterSpacing: '-0.02em',
      color: 'var(--ink-900)',
      whiteSpace: 'nowrap'
    }
  }, "ming", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--terracotta)'
    }
  }, "'"), "s work"));
}
function Btn({
  children,
  variant = 'primary',
  size = 'md',
  icon,
  onClick,
  style = {}
}) {
  const [hover, setHover] = useState(false);
  const base = {
    fontFamily: 'var(--font-sans)',
    fontWeight: 500,
    display: 'inline-flex',
    alignItems: 'center',
    gap: 8,
    borderRadius: 'var(--r-sm)',
    border: '1px solid transparent',
    cursor: 'pointer',
    transition: 'all var(--dur) var(--ease-out)',
    fontSize: size === 'sm' ? 13 : 14.5,
    whiteSpace: 'nowrap',
    padding: size === 'sm' ? '8px 14px' : '11px 19px',
    lineHeight: 1,
    ...style
  };
  const variants = {
    primary: {
      background: hover ? 'var(--terracotta-deep)' : 'var(--terracotta)',
      color: '#FCEFE7',
      boxShadow: 'var(--shadow-xs)'
    },
    secondary: {
      background: hover ? 'var(--sand)' : 'var(--card)',
      color: 'var(--ink-900)',
      borderColor: hover ? 'var(--ink-300)' : 'var(--line-strong)'
    },
    ghost: {
      background: hover ? 'var(--sand)' : 'transparent',
      color: 'var(--ink-700)'
    },
    dark: {
      background: hover ? '#000' : 'var(--ink-900)',
      color: 'var(--paper)',
      borderRadius: 'var(--r-pill)'
    }
  };
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      ...base,
      ...variants[variant]
    }
  }, children, icon && /*#__PURE__*/React.createElement(Icon, {
    name: icon,
    size: 16
  }));
}
function Tag({
  children
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 11,
      color: 'var(--ink-500)',
      border: '1px solid var(--line-strong)',
      borderRadius: 'var(--r-xs)',
      padding: '4px 9px'
    }
  }, children);
}
const STATUS = {
  shipped: {
    label: 'Shipped',
    c: 'var(--success)',
    bg: 'var(--success-soft)'
  },
  live: {
    label: 'Live',
    c: 'var(--terracotta-deep)',
    bg: 'var(--terracotta-wash)'
  },
  wip: {
    label: 'In progress',
    c: 'var(--warning)',
    bg: 'var(--warning-soft)'
  },
  archived: {
    label: 'Archived',
    c: 'var(--ink-500)',
    bg: 'var(--sand)'
  }
};
function StatusChip({
  status
}) {
  const s = STATUS[status] || STATUS.shipped;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontWeight: 500,
      fontSize: 11,
      letterSpacing: '.05em',
      textTransform: 'uppercase',
      padding: '5px 11px',
      borderRadius: 'var(--r-pill)',
      color: s.c,
      background: s.bg,
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: '50%',
      background: s.c
    }
  }), s.label);
}
function Label({
  children,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontWeight: 500,
      fontSize: 12,
      letterSpacing: '.08em',
      textTransform: 'uppercase',
      color: 'var(--ink-400)',
      ...style
    }
  }, children);
}

/* Project data — the portfolio's content. */
const PROJECTS = [{
  id: 'marginalia',
  n: '01',
  name: 'Marginalia',
  year: '2025',
  status: 'live',
  tagline: 'A calmer way to read and annotate.',
  grad: 'linear-gradient(135deg,#C05A36,#CD8A5A)',
  tech: ['Swift', 'CloudKit', 'PDFKit'],
  summary: 'A reading app that keeps your notes in the margins, not in a separate silo. Highlights, marginalia, and backlinks live next to the text and sync quietly across devices.',
  role: 'Design + build · solo',
  points: ['Margin notes anchor to text, not page coordinates — they survive reflow.', 'Local-first store with CloudKit sync; everything works fully offline.', 'A reading view tuned for long sessions: warm paper, generous measure.']
}, {
  id: 'ledger',
  n: '02',
  name: 'Ledger',
  year: '2024',
  status: 'shipped',
  tagline: 'Plain-text personal finance.',
  grad: 'linear-gradient(135deg,#6E7347,#8C9162)',
  tech: ['Rust', 'SQLite', 'TypeScript'],
  summary: 'Double-entry finance stored as human-readable plain text. You own the file; the app is just a fast, friendly lens over it.',
  role: 'Design + build · solo',
  points: ['A Rust core parses and balances ledgers in milliseconds.', 'Every change is a diff you can read, version, and trust.', 'Reports render from the same text the way a spreadsheet never could.']
}, {
  id: 'field-notes',
  n: '03',
  name: 'Field Notes',
  year: '2023',
  status: 'shipped',
  tagline: 'Offline-first journaling.',
  grad: 'linear-gradient(135deg,#4E6C6B,#7E9A93)',
  tech: ['React Native', 'SQLite'],
  summary: 'A journaling app for people who write in the woods. Captures fast, syncs when it can, and never loses a word.',
  role: 'Design + build · solo',
  points: ['Capture-first UI: one tap from launch to writing.', 'Conflict-free sync that merges entries without surprises.', 'Exports to Markdown — your words leave whenever you want.']
}, {
  id: 'kiln',
  n: '04',
  name: 'Kiln',
  year: '2022',
  status: 'archived',
  tagline: 'A tiny static-site baker.',
  grad: 'linear-gradient(135deg,#A2472A,#C08A2C)',
  tech: ['Go', 'Templates'],
  summary: 'A single-binary static site generator I built to publish my own notes. Fast, opinionated, and pleasantly boring.',
  role: 'Design + build · solo',
  points: ['One binary, no config to start — convention over options.', 'Incremental builds that finish before you lift your finger.', 'Powers this very portfolio.']
}];
Object.assign(window, {
  useIcons,
  Icon,
  Logo,
  Btn,
  Tag,
  StatusChip,
  Label,
  PROJECTS,
  React,
  useState,
  useEffect,
  useRef,
  useLayoutEffect
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/portfolio/ui.jsx", error: String((e && e.message) || e) }); }

// ui_kits/webapp/app.jsx
try { (() => {
/* app.jsx — App shell + mount for Marginalia */

function App() {
  const [collection, setCollection] = useState('Library');
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(null);
  useIcons((open ? open.id : collection) + query);
  const docs = DOCS.filter(d => (collection === 'Library' || d.collection === collection) && (query.trim() === '' || (d.title + d.author).toLowerCase().includes(query.toLowerCase())));
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100vh',
      display: 'flex',
      background: 'var(--paper)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement(Sidebar, {
    active: open ? null : collection,
    onSelect: c => {
      setCollection(c);
      setOpen(null);
    }
  }), /*#__PURE__*/React.createElement("main", {
    style: {
      flex: 1,
      display: 'flex',
      flexDirection: 'column',
      minWidth: 0
    }
  }, open ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(Topbar, {
    title: "Reading",
    showSearch: false,
    right: /*#__PURE__*/React.createElement(Btn, {
      variant: "secondary",
      size: "sm",
      iconLeft: "arrow-left",
      onClick: () => setOpen(null),
      style: {
        marginLeft: 'auto'
      }
    }, "Library")
  }), /*#__PURE__*/React.createElement(Reader, {
    d: open
  })) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(Topbar, {
    title: collection,
    query: query,
    onSearch: setQuery,
    right: /*#__PURE__*/React.createElement(Btn, {
      variant: "ghost",
      size: "sm",
      iconLeft: "arrow-up-down"
    }, "Recent")
  }), /*#__PURE__*/React.createElement(Library, {
    docs: docs,
    onOpen: setOpen
  }))));
}
ReactDOM.createRoot(document.getElementById('root')).render(/*#__PURE__*/React.createElement(App, null));
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/webapp/app.jsx", error: String((e && e.message) || e) }); }

// ui_kits/webapp/shell.jsx
try { (() => {
/* shell.jsx — Sidebar + Topbar for Marginalia */

function Sidebar({
  active,
  onSelect
}) {
  return /*#__PURE__*/React.createElement("aside", {
    style: {
      width: 232,
      flex: '0 0 232px',
      background: 'var(--sand)',
      borderRight: '1px solid var(--line)',
      display: 'flex',
      flexDirection: 'column',
      padding: '20px 14px',
      height: '100%',
      boxSizing: 'border-box'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '4px 8px 18px'
    }
  }, /*#__PURE__*/React.createElement(MarkTile, {
    size: 24
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 19,
      letterSpacing: '-0.01em',
      color: 'var(--ink-900)'
    }
  }, "Marginalia")), /*#__PURE__*/React.createElement(Btn, {
    variant: "primary",
    iconLeft: "plus",
    style: {
      justifyContent: 'center',
      marginBottom: 18
    }
  }, "Add reading"), /*#__PURE__*/React.createElement("nav", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 2
    }
  }, COLLECTIONS.map(c => {
    const on = active === c.name;
    return /*#__PURE__*/React.createElement("button", {
      key: c.name,
      onClick: () => onSelect(c.name),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 11,
        padding: '9px 10px',
        borderRadius: 'var(--r-sm)',
        border: 'none',
        cursor: 'pointer',
        background: on ? 'var(--card)' : 'transparent',
        boxShadow: on ? 'var(--shadow-xs)' : 'none',
        color: on ? 'var(--ink-900)' : 'var(--ink-500)',
        fontFamily: 'var(--font-sans)',
        fontSize: 14.5,
        fontWeight: on ? 600 : 500,
        transition: 'all var(--dur)'
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: c.icon,
      size: 17,
      style: {
        color: on ? 'var(--terracotta)' : 'var(--ink-400)'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 1,
        textAlign: 'left'
      }
    }, c.name), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--ink-300)'
      }
    }, c.count));
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 'auto',
      borderTop: '1px solid var(--line)',
      paddingTop: 14,
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 30,
      height: 30,
      borderRadius: '50%',
      background: 'var(--olive)',
      color: '#EEF0E2',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-display)',
      fontSize: 15
    }
  }, "M"), /*#__PURE__*/React.createElement("div", {
    style: {
      lineHeight: 1.2
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 13.5,
      fontWeight: 600,
      color: 'var(--ink-900)'
    }
  }, "Ming"), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 10.5,
      color: 'var(--ink-400)'
    }
  }, "Free plan")), /*#__PURE__*/React.createElement(Icon, {
    name: "settings",
    size: 16,
    style: {
      color: 'var(--ink-400)',
      marginLeft: 'auto'
    }
  })));
}
function Topbar({
  title,
  onSearch,
  query,
  right,
  showSearch = true
}) {
  const [focus, setFocus] = useState(false);
  return /*#__PURE__*/React.createElement("header", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 18,
      padding: '14px 28px',
      borderBottom: '1px solid var(--line)',
      background: 'var(--paper)',
      flex: '0 0 auto'
    }
  }, /*#__PURE__*/React.createElement("h1", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 22,
      letterSpacing: '-0.01em',
      color: 'var(--ink-900)',
      margin: 0
    }
  }, title), showSearch && /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      width: 250,
      background: 'var(--card)',
      border: '1px solid',
      borderColor: focus ? 'var(--terracotta)' : 'var(--line-strong)',
      boxShadow: focus ? '0 0 0 3px var(--terracotta-wash)' : 'none',
      borderRadius: 'var(--r-sm)',
      padding: '8px 11px',
      transition: 'all var(--dur)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "search",
    size: 15,
    style: {
      color: 'var(--ink-400)'
    }
  }), /*#__PURE__*/React.createElement("input", {
    value: query,
    onChange: e => onSearch(e.target.value),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    placeholder: "Search readings",
    style: {
      border: 'none',
      outline: 'none',
      background: 'transparent',
      flex: 1,
      fontFamily: 'var(--font-sans)',
      fontSize: 14,
      color: 'var(--ink-900)'
    }
  })), right);
}
Object.assign(window, {
  Sidebar,
  Topbar
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/webapp/shell.jsx", error: String((e && e.message) || e) }); }

// ui_kits/webapp/ui.jsx
try { (() => {
/* ui.jsx — primitives + data for the Marginalia web app kit */
const {
  useState,
  useEffect,
  useRef,
  useLayoutEffect
} = React;
function useIcons(dep) {
  useLayoutEffect(() => {
    if (window.lucide) window.lucide.createIcons();
  }, [dep]);
}
function Icon({
  name,
  size = 16,
  stroke = 1.75,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("i", {
    "data-lucide": name,
    style: {
      width: size,
      height: size,
      strokeWidth: stroke,
      display: 'inline-flex',
      ...style
    }
  });
}
function MarkTile({
  size = 26
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      width: size,
      height: size,
      borderRadius: size * 0.23,
      background: 'var(--terracotta)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      flex: '0 0 auto'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: size * 0.6,
    height: size * 0.6,
    viewBox: "0 0 48 48",
    fill: "none"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M13 34 V21.5 Q13 15.5 18.75 15.5 Q24.5 15.5 24.5 21.5 V34 M24.5 21.5 Q24.5 15.5 30.25 15.5 Q36 15.5 36 21.5 V34",
    stroke: "#FCFAF3",
    strokeWidth: "3.4",
    strokeLinecap: "round",
    strokeLinejoin: "round"
  })));
}
function Btn({
  children,
  variant = 'primary',
  size = 'md',
  icon,
  iconLeft,
  onClick,
  style = {}
}) {
  const [h, setH] = useState(false);
  const base = {
    fontFamily: 'var(--font-sans)',
    fontWeight: 500,
    display: 'inline-flex',
    alignItems: 'center',
    gap: 7,
    borderRadius: 'var(--r-sm)',
    border: '1px solid transparent',
    cursor: 'pointer',
    whiteSpace: 'nowrap',
    transition: 'all var(--dur) var(--ease-out)',
    fontSize: size === 'sm' ? 13 : 14,
    lineHeight: 1,
    padding: size === 'sm' ? '7px 12px' : '9px 15px',
    ...style
  };
  const v = {
    primary: {
      background: h ? 'var(--terracotta-deep)' : 'var(--terracotta)',
      color: '#FCEFE7',
      boxShadow: 'var(--shadow-xs)'
    },
    secondary: {
      background: h ? 'var(--sand)' : 'var(--card)',
      color: 'var(--ink-900)',
      borderColor: 'var(--line-strong)'
    },
    ghost: {
      background: h ? 'var(--sand)' : 'transparent',
      color: 'var(--ink-700)'
    }
  };
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    onMouseEnter: () => setH(true),
    onMouseLeave: () => setH(false),
    style: {
      ...base,
      ...v[variant]
    }
  }, iconLeft && /*#__PURE__*/React.createElement(Icon, {
    name: iconLeft,
    size: 15
  }), children, icon && /*#__PURE__*/React.createElement(Icon, {
    name: icon,
    size: 15
  }));
}
function Label({
  children,
  style = {}
}) {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      fontWeight: 500,
      fontSize: 11,
      letterSpacing: '.08em',
      textTransform: 'uppercase',
      color: 'var(--ink-400)',
      whiteSpace: 'nowrap',
      ...style
    }
  }, children);
}
const NOTE_C = {
  terracotta: 'var(--terracotta)',
  olive: 'var(--olive)',
  clay: 'var(--clay)'
};

/* Reading library. `body` is an array of paragraphs; a paragraph can carry inline
   highlights via {h:'terracotta', text:'...'} segments and an attached margin note. */
const DOCS = [{
  id: 'attention',
  title: 'On Paying Attention',
  author: 'Ada Reyes',
  source: 'essays.adareyes.com',
  minutes: 12,
  progress: 64,
  collection: 'Essays',
  notes: 3,
  color: 'terracotta',
  body: [{
    p: 'Attention is the rarest and purest form of generosity. We talk about it like a resource to be spent, but it behaves more like a muscle — it tires, it recovers, and it grows in the direction we point it.'
  }, {
    p: 'The tools we use to read have quietly become tools that read us back. A margin used to be a private place, the one part of a book that belonged entirely to you.',
    hi: 'A margin used to be a private place',
    note: {
      color: 'terracotta',
      text: 'This is the whole thesis of Marginalia, really. The margin as the last private space.'
    }
  }, {
    p: 'What we annotate, we remember. What we merely highlight, we forget twice — once when we stop reading, and again when we never return.',
    hi: 'What we annotate, we remember',
    note: {
      color: 'olive',
      text: 'Active recall > passive highlighting. Worth designing the friction to favor notes.'
    }
  }, {
    p: 'So the question is not how to read more, but how to read in a way that leaves a mark on us — and lets us leave a mark in return.'
  }]
}, {
  id: 'small-tools',
  title: 'In Praise of Small Tools',
  author: 'Ming',
  source: 'mings.work/notes',
  minutes: 6,
  progress: 100,
  collection: 'Notes',
  notes: 2,
  color: 'olive',
  body: [{
    p: 'A small tool fits in your head. You can hold all of it at once — every screen, every state, every decision. That wholeness is a feeling users can sense even when they can\u2019t name it.'
  }, {
    p: 'Big software apologizes for itself with onboarding. Small software just works, because there is nothing to onboard into.',
    hi: 'Small software just works',
    note: {
      color: 'olive',
      text: 'The best onboarding is no onboarding.'
    }
  }]
}, {
  id: 'plain-text',
  title: 'The Case for Plain Text',
  author: 'J. Okonkwo',
  source: 'plaintextproject.org',
  minutes: 9,
  progress: 22,
  collection: 'Essays',
  notes: 1,
  color: 'clay',
  body: [{
    p: 'Plain text outlives its tools. The note you wrote in 1994 still opens today, while the app you wrote it in is a fossil. Formats are promises, and plain text keeps them.',
    hi: 'Formats are promises',
    note: {
      color: 'clay',
      text: 'Use as the tagline for Ledger.'
    }
  }, {
    p: 'Own your file, and you own your future. Everything else is a lease.'
  }]
}, {
  id: 'walking',
  title: 'Walking as a Way of Knowing',
  author: 'Lena Hart',
  source: 'orionmagazine.org',
  minutes: 15,
  progress: 0,
  collection: 'Saved',
  notes: 0,
  color: 'terracotta',
  body: [{
    p: 'To walk is to think with the whole body. The pace of the feet sets the pace of the mind, and a problem that resists the desk often dissolves on a path.'
  }]
}];
const COLLECTIONS = [{
  name: 'Library',
  icon: 'library',
  count: 4
}, {
  name: 'Essays',
  icon: 'file-text',
  count: 2
}, {
  name: 'Notes',
  icon: 'pen-line',
  count: 1
}, {
  name: 'Saved',
  icon: 'bookmark',
  count: 1
}];
Object.assign(window, {
  useState,
  useEffect,
  useRef,
  useLayoutEffect,
  useIcons,
  Icon,
  MarkTile,
  Btn,
  Label,
  NOTE_C,
  DOCS,
  COLLECTIONS
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/webapp/ui.jsx", error: String((e && e.message) || e) }); }

// ui_kits/webapp/views.jsx
try { (() => {
/* views.jsx — Library list + Reader for Marginalia */

function Progress({
  value,
  color = 'terracotta',
  w = 120
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      width: w,
      height: 5,
      background: 'var(--sand-deep)',
      borderRadius: 999,
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: value + '%',
      height: '100%',
      background: NOTE_C[color],
      borderRadius: 999
    }
  }));
}
function ReadingRow({
  d,
  onOpen
}) {
  const [h, setH] = useState(false);
  return /*#__PURE__*/React.createElement("button", {
    onClick: () => onOpen(d),
    onMouseEnter: () => setH(true),
    onMouseLeave: () => setH(false),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 18,
      width: '100%',
      textAlign: 'left',
      cursor: 'pointer',
      padding: '16px 18px',
      borderRadius: 'var(--r-md)',
      border: '1px solid',
      borderColor: h ? 'var(--line-strong)' : 'transparent',
      background: h ? 'var(--card)' : 'transparent',
      boxShadow: h ? 'var(--shadow-sm)' : 'none',
      transition: 'all var(--dur)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 46,
      height: 58,
      borderRadius: 'var(--r-sm)',
      flex: '0 0 auto',
      background: NOTE_C[d.color],
      boxShadow: 'var(--shadow-xs)',
      position: 'relative'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      inset: 0,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-display)',
      fontSize: 22,
      color: 'rgba(255,255,255,.92)'
    }
  }, d.title[0])), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 19,
      letterSpacing: '-0.01em',
      color: 'var(--ink-900)',
      marginBottom: 3
    }
  }, d.title), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 13.5,
      color: 'var(--ink-500)'
    }
  }, d.author, " \xB7 ", d.source)), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'flex-end',
      gap: 7,
      flex: '0 0 auto'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, d.notes > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      fontFamily: 'var(--font-mono)',
      fontSize: 11,
      color: 'var(--ink-400)'
    }
  }, /*#__PURE__*/React.createElement(Icon, {
    name: "message-square",
    size: 13
  }), d.notes), /*#__PURE__*/React.createElement(Label, {
    style: {
      color: 'var(--ink-300)'
    }
  }, d.minutes, " min")), /*#__PURE__*/React.createElement(Progress, {
    value: d.progress,
    color: d.color,
    w: 110
  })));
}
function Library({
  docs,
  onOpen
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto',
      padding: '24px 16px 40px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 760,
      margin: '0 auto'
    }
  }, /*#__PURE__*/React.createElement(Label, {
    style: {
      padding: '0 18px'
    }
  }, docs.length, " readings \xB7 1 in progress"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 12,
      display: 'flex',
      flexDirection: 'column'
    }
  }, docs.length === 0 ? /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      color: 'var(--ink-400)',
      padding: 18
    }
  }, "Nothing here yet.") : docs.map((d, i) => /*#__PURE__*/React.createElement(React.Fragment, {
    key: d.id
  }, i > 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      height: 1,
      background: 'var(--line)',
      margin: '0 18px'
    }
  }), /*#__PURE__*/React.createElement(ReadingRow, {
    d: d,
    onOpen: onOpen
  }))))));
}
function MarginNote({
  note
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--card)',
      border: '1px solid var(--line)',
      borderLeft: '2px solid ' + NOTE_C[note.color],
      borderRadius: 'var(--r-sm)',
      padding: '11px 13px',
      boxShadow: 'var(--shadow-xs)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      marginBottom: 6
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 16,
      height: 16,
      borderRadius: '50%',
      background: 'var(--olive)',
      color: '#EEF0E2',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-display)',
      fontSize: 10
    }
  }, "M"), /*#__PURE__*/React.createElement(Label, {
    style: {
      fontSize: 10
    }
  }, "Margin note")), /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 13.5,
      lineHeight: 1.5,
      color: 'var(--ink-700)',
      margin: 0
    }
  }, note.text));
}
function Para({
  block
}) {
  let content = block.p;
  if (block.hi) {
    const idx = block.p.indexOf(block.hi);
    if (idx >= 0) {
      content = [block.p.slice(0, idx), /*#__PURE__*/React.createElement("mark", {
        key: "h",
        style: {
          background: 'var(--terracotta-wash)',
          color: 'inherit',
          boxShadow: 'inset 0 -2px 0 ' + NOTE_C[block.note ? block.note.color : 'terracotta'],
          borderRadius: 2,
          padding: '0 1px'
        }
      }, block.hi), block.p.slice(idx + block.hi.length)];
    }
  }
  return /*#__PURE__*/React.createElement("p", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 19,
      lineHeight: 1.7,
      color: 'var(--ink-800, var(--ink-700))',
      margin: 0
    }
  }, content);
}
function Reader({
  d
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflow: 'auto',
      padding: '36px 28px 80px',
      background: 'var(--paper)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: 920,
      margin: '0 auto'
    }
  }, /*#__PURE__*/React.createElement(Label, {
    style: {
      color: 'var(--terracotta-deep)'
    }
  }, d.collection, " \xB7 ", d.minutes, " min read"), /*#__PURE__*/React.createElement("h2", {
    style: {
      fontFamily: 'var(--font-display)',
      fontSize: 'clamp(30px,4vw,46px)',
      letterSpacing: '-0.02em',
      color: 'var(--ink-900)',
      margin: '12px 0 8px',
      lineHeight: 1.06
    }
  }, d.title), /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-sans)',
      fontSize: 15,
      color: 'var(--ink-500)',
      marginBottom: 32
    }
  }, d.author, " \xB7 ", d.source), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 22
    }
  }, d.body.map((block, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 250px',
      gap: 28,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement(Para, {
    block: block
  }), /*#__PURE__*/React.createElement("div", null, block.note && /*#__PURE__*/React.createElement(MarginNote, {
    note: block.note
  })))))));
}
Object.assign(window, {
  Progress,
  ReadingRow,
  Library,
  MarginNote,
  Para,
  Reader
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/webapp/views.jsx", error: String((e && e.message) || e) }); }

})();
