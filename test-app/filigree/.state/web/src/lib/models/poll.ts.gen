import { client, type ModelDefinition } from "filigree-svelte";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export type PollId = string;

export const PollSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	question: z.string(),
	answers: z.any(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Poll = z.infer<typeof PollSchema>;
export const PollListResultSchema = PollSchema;
export type PollListResult = Poll;
export const PollPopulatedGetResultSchema = PollSchema;
export type PollPopulatedGetResult = Poll;
export const PollPopulatedListResultSchema = PollSchema;
export type PollPopulatedListResult = Poll;
export const PollCreateResultSchema = PollSchema;
export type PollCreateResult = Poll;

export const PollCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	question: z.string(),
	answers: z.any(),
	post_id: z.string().uuid(),
});

export type PollCreatePayloadAndUpdatePayload = z.infer<
	typeof PollCreatePayloadAndUpdatePayloadSchema
>;
export const PollCreatePayloadSchema = PollCreatePayloadAndUpdatePayloadSchema;
export type PollCreatePayload = PollCreatePayloadAndUpdatePayload;
export const PollUpdatePayloadSchema = PollCreatePayloadAndUpdatePayloadSchema;
export type PollUpdatePayload = PollCreatePayloadAndUpdatePayload;

export const baseUrl = "polls";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const PollModel: ModelDefinition<typeof PollSchema> = {
	name: "Poll",
	plural: "Polls",
	baseUrl,
	urls,
	schema: PollSchema,
	createSchema: PollCreatePayloadSchema,
	updateSchema: PollUpdatePayloadSchema,
	fields: [
		{
			name: "id",
			type: "uuid",
			label: "Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "organization_id",
			type: "uuid",
			label: "Organization Id",
			constraints: {
				required: true,
			},
		},
		{
			name: "updated_at",
			type: "date-time",
			label: "Updated At",
			constraints: {
				required: true,
			},
		},
		{
			name: "created_at",
			type: "date-time",
			label: "Created At",
			constraints: {
				required: true,
			},
		},
		{
			name: "question",
			type: "text",
			label: "Question",
			constraints: {
				required: true,
			},
		},
		{
			name: "answers",
			type: "object",
			label: "Answers",
			constraints: {
				required: true,
			},
		},
		{
			name: "post_id",
			type: "uuid",
			label: "Post Id",
			constraints: {
				required: true,
			},
		},
	],
};
