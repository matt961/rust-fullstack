/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{html,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
    container: {
      center: true
    }
  },
  plugins: [
    require("@tailwindcss/forms")
  ],
}
