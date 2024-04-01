import colors from 'tailwindcss/colors';
import { fontFamily } from 'tailwindcss/defaultTheme';
import svelteUx from 'svelte-ux/plugins/tailwind.cjs';

/** @type {import('tailwindcss').Config} */
const config = {
  darkMode: ['class'],
  content: ['./src/**/*.{html,js,svelte,ts}', './node_modules/svelte-ux/**/*.{svelte,js}'],
  plugins: [svelteUx],
  safelist: ['dark'],
  theme: {
    extend: {
      fontFamily: {
        sans: [...fontFamily.sans],
      },
    },
  },
  ux: {
  },
};

export default config;
