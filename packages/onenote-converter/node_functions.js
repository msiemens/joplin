const fs = require('node:fs');
const path = require('node:path');

function normalize(filePath) {
	if (!isWindows()) return path.normalize(filePath);

	const abs = path.resolve(filePath);
	const s = abs.replace(/\//g, '\\');

	if (s.startsWith('\\\\?\\')) {
		return s;
	}

	if (s.startsWith('\\\\')) {
		// UNC share path → \\?\UNC\server\share\...
		return `\\\\?\\UNC\\${s.substring(2)}`;
	} else {
		return `\\\\?\\${s}`;
	}
}

function makeDir(filePath) {
	fs.mkdirSync(filePath, { recursive: true });
}

function isDirectory(filePath) {
	filePath = normalize(filePath);
	if (!fs.existsSync(filePath)) return false;

	return fs.lstatSync(filePath).isDirectory();
}

function readDir(filePath) {
	filePath = normalize(filePath);
	const dirContents = fs.readdirSync(filePath, { withFileTypes: true });

	return dirContents.map(entry => filePath + path.sep + entry.name);
}

function normalizeAndWriteFile(filePath, data) {
	filePath = path.normalize(filePath);

	fs.writeFileSync(filePath, data);
}

function openFileForReading(filePath) {
	filePath = normalize(filePath);
	return fs.openSync(filePath, 'r');
}

function openFileForWriting(filePath) {
	filePath = path.normalize(filePath);
	return fs.openSync(filePath, 'w');
}

function writeChunk(fd, data) {
	fs.writeSync(fd, data, 0, data.length);
}

function closeFile(fd) {
	fs.closeSync(fd);
}

function fileSize(filePath) {
	filePath = normalize(filePath);
	return fs.statSync(filePath).size;
}

function readFileChunk(fd, offset, length) {
	const buf = Buffer.alloc(length);
	const bytesRead = fs.readSync(fd, buf, 0, length, offset);
	return bytesRead === length ? buf : buf.subarray(0, bytesRead);
}

function isWindows() {
	return process.platform === 'win32';
}

module.exports = {
	makeDir,
	isDirectory,
	readDir,
	normalizeAndWriteFile,
	openFileForReading,
	openFileForWriting,
	writeChunk,
	closeFile,
	fileSize,
	readFileChunk,
	isWindows,
};
