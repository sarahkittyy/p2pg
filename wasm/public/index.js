import init from '../app.js';

let url = new URL('../app_bg.wasm', import.meta.url);

let response = await fetch(url);

const reader = response.body.getReader();
// length of wasm download
const total_bytes = parseInt(response.headers.get('Content-Length'));

let svg_circle = document.getElementById('loading-circle');
let progress_bar = document.getElementById('progress-bar');
let loading_text = document.getElementById('loading-text');
let resolution_text = document.getElementById('resolution-warning');

function bouncy_text(text, index) {
	return text.split('').map((char, i) => i === index ? char.toUpperCase() : char).join('');
}

let bouncy_text_interval = (() => {
	let loading_string = "loading";
	let capitalized_index = 0;
	return setInterval(() => {
		loading_text.innerText = bouncy_text(loading_string, capitalized_index % loading_string.length);
		capitalized_index++;
	}, 150);
})();

let recv_bytes = 0;
let chunks = [];
while(1) {
	const { done, value } = await reader.read();
	if (done) break;

	chunks.push(value);
	recv_bytes += value.length;

	update_loading_bar(recv_bytes / total_bytes);
}

let allChunks = new Uint8Array(recv_bytes);
let pos = 0;
for(let chunk of chunks) {
	allChunks.set(chunk, pos);
	pos += chunk.length;
}

init(allChunks);
resolution_text.hidden = true;

function update_loading_bar(t) {
	function sdo(t) {
		return 440 - (440 * t);
	}
	svg_circle.style.strokeDashoffset = sdo(t);
	if (t > 0.99) {
		progress_bar.style.display = 'none';
		clearInterval(bouncy_text_interval);
	}
}