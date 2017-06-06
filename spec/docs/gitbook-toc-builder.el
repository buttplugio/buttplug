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
               (setq toc-header-link (format "%s- [[%s#%s][%s]]\n"
                     (make-string (* (- heading-level 2) 2) ? )
                     current-export-file
                     (s-dashed-words heading-name)
                     heading-name))
               (setq current-toc (concat current-toc toc-header-link)))))
       "-noexport" 'region))
    current-toc))


