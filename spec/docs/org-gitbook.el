(defvar org-gitbook-output-directory "./build"
  "Directory that org-gitbook will export files to")

;; export headlines to separate files
;; http://emacs.stackexchange.com/questions/2259/how-to-export-top-level-headings-of-org-mode-buffer-to-separate-files
(defun org-export-gitbook ()
  "Export all subtrees that are *not* tagged with :noexport: to
  separate files.

  Subtrees that do not have the :EXPORT_FILE_NAME: property set
  are exported to a filename derived from the headline text."
  (interactive)
  (save-buffer)
  (let ((modifiedp (buffer-modified-p)))
    (save-excursion
      (goto-char (point-min))
      (goto-char (re-search-forward "^*"))
      (set-mark (line-beginning-position))
      (goto-char (point-max))
      (if (and org-gitbook-output-directory (not (file-accessible-directory-p org-gitbook-output-directory)))
          (mkdir org-gitbook-output-directory))
      (org-map-entries
       (lambda ()
         (let ((export-file (org-entry-get (point) "EXPORT_FILE_NAME")))
           (unless export-file
             (org-set-property
              "EXPORT_FILE_NAME"
              (replace-regexp-in-string " " "_" (nth 4 (org-heading-components)))))
           (setq tempfile (org-entry-get (point) "EXPORT_FILE_NAME"))
           (if org-gitbook-output-directory
               (org-set-property
                "EXPORT_FILE_NAME" (concat org-gitbook-output-directory "/" tempfile)))
           (deactivate-mark)
           (org-gfm-export-to-markdown nil t)
           (org-set-property "EXPORT_FILE_NAME" tempfile)
           (unless export-file (org-delete-property "EXPORT_FILE_NAME"))
           (set-buffer-modified-p modifiedp)))
       "-noexport" 'region-start-level))))

(defun org-build-gitbook-toc ()
  (interactive)
  (save-excursion
    (set-mark (point-min))
    (goto-char (point-max))
    (setq current-export-file "")
    (setq current-toc "")
    (org-map-entries
     (lambda ()
       (let ((export-file (org-entry-get (point) "EXPORT_FILE_NAME"))
             (heading-level (nth 0 (org-heading-components)))
             (heading-name (nth 4 (org-heading-components))))
         (if export-file
             (setq current-export-file export-file))
         (if (> heading-level 1)
             (progn
               (setq current-toc (concat current-toc
                                         (format "%s- %s\n"
                                                 (make-string (* (- heading-level 2) 2) ? )
                                                 (org-make-link-string
                                                  (concat "file:" current-export-file "#" (s-dashed-words heading-name))
                                                  heading-name))))))))
     "-noexport" 'region))
  current-toc)
