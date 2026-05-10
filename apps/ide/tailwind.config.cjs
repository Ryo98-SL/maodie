const { join } = require("node:path");

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [join(__dirname, "index.html"), join(__dirname, "src/**/*.{ts,tsx}")],
  theme: {
    extend: {}
  },
  plugins: []
};
