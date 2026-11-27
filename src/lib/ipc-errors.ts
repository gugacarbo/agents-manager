import type { CoreError } from "@/bindings/tauri";

const SSH_ERROR_HINTS: Record<string, string> = {
	"host-key": "Verify and approve the host key using OpenSSH outside this app.",
	authentication:
		"Configure non-interactive authentication in OpenSSH outside this app.",
	"client-unavailable": "Install the OpenSSH client and ensure ssh is in PATH.",
	timeout: "Check connectivity and the OpenSSH configuration, then try again.",
	connection:
		"Check the host, port, and OpenSSH configuration, then try again.",
};

const CORE_UNIT_MESSAGES: Record<Extract<CoreError, string>, string> = {
	ExternalConflict: "File changed externally.",
	SecretConfirmationRequired: "Literal secret confirmation required.",
	ProtectedField:
		"Project settings in this shared file cannot be changed remotely.",
	RemoteDestinationChanged:
		"The authenticated destination changed. Open the file again.",
	RemoteUnconfirmed: "The remote save could not be confirmed.",
};

function readStringProperty(value: unknown, key: string): string | null {
	if (typeof value !== "object" || value === null) {
		return null;
	}
	const record = value as Record<string, unknown>;
	return typeof record[key] === "string" ? record[key] : null;
}

export function formatCoreErrorMessage(err: unknown): string | null {
	if (typeof err === "string") {
		return Object.hasOwn(CORE_UNIT_MESSAGES, err)
			? CORE_UNIT_MESSAGES[err as Extract<CoreError, string>]
			: null;
	}

	if (typeof err !== "object" || err === null) {
		return null;
	}

	const record = err as Record<string, unknown>;

	const ioMessage =
		readStringProperty(record.IoError, "message") ??
		readStringProperty(record.Io, "message");
	if (ioMessage !== null) {
		return `I/O error: ${ioMessage}`;
	}

	const serializationMessage = readStringProperty(
		record.Serialization,
		"message",
	);
	if (serializationMessage !== null) {
		return `Serialization error: ${serializationMessage}`;
	}

	const invalidPath = readStringProperty(record.InvalidPath, "path");
	if (invalidPath !== null) {
		return `Invalid path: ${invalidPath}`;
	}

	const notFoundTarget = readStringProperty(record.NotFound, "target");
	if (notFoundTarget !== null) {
		return `Not found: ${notFoundTarget}`;
	}

	const permissionDeniedMessage = readStringProperty(
		record.PermissionDenied,
		"message",
	);
	if (permissionDeniedMessage !== null) {
		return `Permission denied: ${permissionDeniedMessage}`;
	}

	const pathNotAllowed = readStringProperty(record.PathNotAllowed, "path");
	if (pathNotAllowed !== null) {
		return `Path not allowed: ${pathNotAllowed}`;
	}

	const syntaxMessage = readStringProperty(record.SyntaxError, "message");
	if (syntaxMessage !== null) {
		return `Syntax error: ${syntaxMessage}`;
	}

	const duplicateRootPath = readStringProperty(
		record.DuplicateProject,
		"root_path",
	);
	if (duplicateRootPath !== null) {
		return `Project already registered: ${duplicateRootPath}`;
	}

	const sshMessage = readStringProperty(record.SshError, "message");
	if (sshMessage !== null) {
		const kind = readStringProperty(record.SshError, "kind") ?? "";
		const hint = Object.hasOwn(SSH_ERROR_HINTS, kind)
			? SSH_ERROR_HINTS[kind]
			: undefined;
		return [sshMessage, hint].filter(Boolean).join(" ") || "SSH error";
	}

	return null;
}

export function formatCoreError(err: unknown): Error {
	if (err instanceof Error) {
		return err;
	}
	const coreMessage = formatCoreErrorMessage(err);
	if (coreMessage !== null) {
		return new Error(coreMessage);
	}
	if (typeof err === "string") {
		return new Error(err);
	}
	return new Error("Unknown IPC error");
}
