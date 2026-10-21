// Everything in this file is MOCK data for UI the backend cannot serve yet
// (users, reviewers, activity, graph list, auth). Real task data comes from api.ts.
// Each gap is listed in TODO.md, section "Дыры API для нового UI".

export type Member = { name: string; initials: string; color: string; role?: string };

/** MOCK: the signed-in user. There is no auth or user endpoint. */
export const CURRENT_USER = {
	name: 'Andrei Chirkin',
	email: 'andruha.chirkin@gmail.com',
	role: 'Engineering lead',
	memberSince: 'Mar 2026'
};

/** MOCK: members of every graph. Task.users holds free-form strings, there is no user list. */
export const MEMBERS: Member[] = [
	{ name: 'Maya Okafor', initials: 'MO', color: '#0F766E' },
	{ name: 'Ilya Petrov', initials: 'IP', color: '#7A3FC4' },
	{ name: 'Sofia Lindqvist', initials: 'SL', color: '#B4530A' },
	{ name: 'Andrei Chirkin', initials: 'AC', color: '#2349C7' }
];
