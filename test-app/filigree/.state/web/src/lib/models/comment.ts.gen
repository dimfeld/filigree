import type { ModelDefinition } from "filigree-web";
import { z } from "zod";
import { ObjectPermission } from "../model_types.js";

export const CommentSchema = z.object({
	id: z.string().uuid(),
	organization_id: z.string().uuid(),
	updated_at: z.string().datetime(),
	created_at: z.string().datetime(),
	body: z.string(),
	post_id: z.string().uuid(),
	_permission: ObjectPermission,
});

export type Comment = z.infer<typeof CommentSchema>;
export const CommentPopulatedGetSchema = CommentSchema;
export type CommentPopulatedGet = Comment;
export const CommentPopulatedListSchema = CommentSchema;
export type CommentPopulatedList = Comment;
export const CommentCreateResultSchema = CommentSchema;
export type CommentCreateResult = Comment;

export const CommentCreatePayloadAndUpdatePayloadSchema = z.object({
	id: z.string().uuid().optional(),
	body: z.string(),
	post_id: z.string().uuid(),
});

export type CommentCreatePayloadAndUpdatePayload = z.infer<
	typeof CommentCreatePayloadAndUpdatePayloadSchema
>;
export const CommentCreatePayloadSchema =
	CommentCreatePayloadAndUpdatePayloadSchema;
export type CommentCreatePayload = CommentCreatePayloadAndUpdatePayload;
export const CommentUpdatePayloadSchema =
	CommentCreatePayloadAndUpdatePayloadSchema;
export type CommentUpdatePayload = CommentCreatePayloadAndUpdatePayload;

export const CommentModel: ModelDefinition<typeof CommentSchema> = {
	name: "Comment",
	plural: "Comments",
	url: "comments",
	schema: CommentSchema,
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
			name: "body",
			type: "text",
			label: "Body",
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
