const fs = require('node:fs');
const path = require('node:path');

function makeDir(filepath) {
	fs.mkdirSync(filepath, { recursive: true });
}

function isDirectory(filepath) {
	if (!fs.existsSync(filepath)) return false;

	return fs.lstatSync(filepath).isDirectory();
}

function readDir(filepath) {
	const dirContents = fs.readdirSync(filepath, { withFileTypes: true });

	return dirContents.map(entry => filepath + path.sep + entry.name);
}

function normalizeAndWriteFile(filePath, data) {
	filePath = path.normalize(filePath);

	fs.writeFileSync(filePath, data);
}

function openFileForReading(filePath) {
	filePath = path.normalize(filePath);
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
	filePath = path.normalize(filePath);
	return fs.statSync(filePath).size;
}

function readFileChunk(fd, offset, length) {
	const buf = Buffer.alloc(length);
	const bytesRead = fs.readSync(fd, buf, 0, length, offset);
	return bytesRead === length ? buf : buf.subarray(0, bytesRead);
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
};
