/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["src/**/*.rs"],
  },
  theme: {
    extend: {
      colors: {
        paper: "#fdfbf7",
        pencil: "#2d2d2d",
        erased: "#e5e0d8",
        marker: "#ff4d4d",
        pen: "#2d5da1",
      },
      fontFamily: {
        kalam: ["Kalam", "cursive"],
        hand: ["Patrick Hand", "cursive"],
      },
      boxShadow: {
        hard: "4px 4px 0px 0px #2d2d2d",
        "hard-sm": "2px 2px 0px 0px #2d2d2d",
        "hard-lg": "8px 8px 0px 0px #2d2d2d",
        "hard-hover": "2px 2px 0px 0px #2d2d2d",
      },
      borderRadius: {
        wobbly: "255px 15px 225px 15px / 15px 225px 15px 255px",
        "wobbly-md": "15px 225px 15px 255px / 225px 15px 255px 15px",
      },
    },
  },
  plugins: [],
};
