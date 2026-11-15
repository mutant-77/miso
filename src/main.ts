import { mount } from 'svelte'
import App from './App.svelte'
import './fonts.css'

const start = () => mount(App, { target: document.getElementById('app')! })

// Antes de montar: las columnas miden el paso con la fuente definitiva.
document.fonts.load('1em Literata', 'aáñЖ').then(start, start)
