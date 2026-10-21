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

export type MockGraph = {
	name: string;
	goal: string;
	done: number;
	total: number;
	color: string;
	blocked: number;
	updated: string;
};

/** MOCK: example graphs for the home screen. There is no "list graphs" endpoint. */
export const MOCK_GRAPHS: MockGraph[] = [
	{
		name: 'Platform storage',
		goal: 'Zero-downtime failover',
		done: 12,
		total: 18,
		color: '#0F766E',
		blocked: 0,
		updated: 'Updated yesterday'
	},
	{
		name: 'Q4 growth',
		goal: 'Reach 50k active users',
		done: 5,
		total: 24,
		color: '#7A3FC4',
		blocked: 5,
		updated: 'Updated 2 h ago'
	},
	{
		name: 'Security audit',
		goal: 'Pass external audit',
		done: 8,
		total: 9,
		color: '#B4530A',
		blocked: 0,
		updated: 'Updated 3 days ago'
	}
];

/** MOCK: starting points offered on the new-graph screen. There are no templates in the backend. */
export const GRAPH_TEMPLATES = ['Software release', 'Hiring pipeline', 'Incident response'];

export type ActivityEntry = { who: string; what: string; when: string };

/** MOCK: change log of a task. The backend keeps no history yet. */
export function mockActivity(): ActivityEntry[] {
	return [
		{ who: 'Ilya Petrov', what: 'moved a blocker to Review', when: 'Today, 10:42' },
		{ who: 'Andrei Chirkin', what: 'changed the status', when: 'Yesterday, 17:05' },
		{ who: 'Andrei Chirkin', what: 'added a blocker', when: 'Oct 15, 11:20' },
		{ who: 'Andrei Chirkin', what: 'created this task', when: 'Oct 14, 16:41' }
	];
}

/** MOCK: every task is verified by this person. There is no reviewer field. */
export const MOCK_REVIEWER = 'Sofia Lindqvist';
