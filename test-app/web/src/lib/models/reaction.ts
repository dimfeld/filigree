import { client, type ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const ReactionSchema = z.object({
	id: z.string(),
	organization_id: z.string(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	type: z.string(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Reaction = z.infer<typeof ReactionSchema>;
export const ReactionPopulatedGetSchema = ReactionSchema;
export type ReactionPopulatedGet = Reaction;
export const ReactionPopulatedListSchema = ReactionSchema;
export type ReactionPopulatedList = Reaction;
export const ReactionCreateResultSchema = ReactionSchema;
export type ReactionCreateResult = Reaction;

export const ReactionCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().optional(),
	type: z.string(),
	post_id: z.string().uuid(),
});

export type ReactionCreatePayloadAndUpdatePayload = z.infer<
	typeof ReactionCreatePayloadAndUpdatePayloadSchema
>;
export const ReactionCreatePayloadSchema =
	ReactionCreatePayloadAndUpdatePayloadSchema;
export type ReactionCreatePayload = ReactionCreatePayloadAndUpdatePayload;
export const ReactionUpdatePayloadSchema =
	ReactionCreatePayloadAndUpdatePayloadSchema;
export type ReactionUpdatePayload = ReactionCreatePayloadAndUpdatePayload;

export const baseUrl = "reactions";
export const urlWithId = (id: string) => `${baseUrl}/${id}`;

export const urls = {
	create: baseUrl,
	list: baseUrl,
	get: urlWithId,
	update: urlWithId,
	delete: urlWithId,
};

export const ReactionModel: ModelDefinition<typeof ReactionSchema> = {
	name: "Reaction",
	plural: "Reactions",
	baseUrl,
	urls,
	schema: ReactionSchema,
	createSchema: ReactionCreatePayloadSchema,
	updateSchema: ReactionUpdatePayloadSchema,
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
			name: "type",
			type: "text",
			label: "Type",
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
