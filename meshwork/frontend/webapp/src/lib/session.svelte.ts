import { browser } from '$app/environment';
import { api, ApiError, type Actor } from './api';

const KEY = 'meshwork.token';

/**
 * Who the web app acts as: an actor token pasted on the "You" page, kept in this browser's
 * localStorage, and the actor it belongs to. Without one the app works anonymously, as before.
 */
class Session {
	token = $state('');
	/** The signed-in actor; null when signed out or not checked yet. */
	me = $state.raw<Actor | null>(null);
	/** The stored token has been checked (or there was none). */
	ready = $state(false);
	/** Set when a stored token stopped working and was dropped. */
	expired = $state(false);

	constructor() {
		if (!browser) return;
		try {
			this.token = localStorage.getItem(KEY) ?? '';
		} catch {
			// storage blocked: stay signed out
		}
		api.setToken(this.token);
		api.waitFor(this.check());
	}

	private async check() {
		if (this.token) {
			try {
				this.me = await api.me(this.token);
			} catch (error) {
				// a revoked token would make every request fail: drop it and carry on signed out
				if (error instanceof ApiError && error.status === 401) {
					this.signOut();
					this.expired = true;
				} else {
					console.warn('Cannot check the stored token:', error);
				}
			}
		}
		this.ready = true;
	}

	/** Checks the token with the backend and keeps it. Returns an error message, '' on success. */
	async signIn(token: string): Promise<string> {
		const trimmed = token.trim();
		if (!trimmed) return 'Paste a token first.';
		try {
			this.me = await api.me(trimmed);
		} catch (error) {
			return error instanceof Error ? error.message : String(error);
		}
		this.token = trimmed;
		this.expired = false;
		api.setToken(trimmed);
		try {
			localStorage.setItem(KEY, trimmed);
		} catch {
			// works for this tab only
		}
		return '';
	}

	signOut() {
		this.token = '';
		this.me = null;
		api.setToken('');
		try {
			localStorage.removeItem(KEY);
		} catch {
			// nothing stored
		}
	}
}

export const session = new Session();
